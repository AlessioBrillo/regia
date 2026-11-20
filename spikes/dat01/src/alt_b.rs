//! Alternative B: SQLite is the source of truth. Every write is an immutable op carrying the
//! version vector its author had seen; visible state is derived from the ops. Sync sends the ops
//! the peer lacks, found by comparing version vectors (explicit operation log, custom sync).

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    path::Path,
};

use rusqlite::{Connection, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use crate::{
    CIRCLE_OBJ, Circle, Conflict, DELETED, Device, Error, Hlc, Index, MOVED, Notice, ObjView, Op,
    Quarantined, Reason, Replica, Schema, Snapshot, index_entry, parse_revoked, pick,
    revoked_field,
};

type Vv = BTreeMap<Device, u64>;

/// One immutable write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireOp {
    /// Author.
    pub dev: Device,
    /// Position in the author's log for this circle (1, 2, 3, ... without holes).
    pub seq: u64,
    /// Stamp, [`Hlc`] text.
    pub hlc: String,
    /// Object id.
    pub obj: String,
    /// Field (or the delete register).
    pub field: String,
    /// Value.
    pub value: String,
    /// Everything the author had seen of this circle before this op.
    pub vv: Vv,
}

#[derive(Serialize, Deserialize)]
enum Msg {
    Have(Vv),
    Ops(Vec<WireOp>),
}

/// Session state: what the peer said it has, and what we already sent.
#[derive(Default)]
pub struct BSync {
    peer_vv: Option<Vv>,
    sent_have: bool,
    sent_ops: bool,
}

/// Replica backed by one SQLite database.
pub struct AltB {
    device: Device,
    circles: BTreeSet<Circle>,
    conn: Connection,
    clock: Hlc,
    now: Option<u64>,
    index: Index,
    schema: Schema,
}

/// Highest author sequence per device in `circle`. Takes a connection so a write can read it
/// inside its own transaction.
fn vv_on(conn: &Connection, circle: Circle) -> Vv {
    let Ok(mut st) =
        conn.prepare_cached("SELECT dev, MAX(seq) FROM ops WHERE circle = ?1 GROUP BY dev")
    else {
        return Vv::new();
    };
    st.query_map([db(circle)], |r| {
        Ok((
            r.get::<_, Device>(0)?,
            u64::try_from(r.get::<_, i64>(1)?).unwrap_or(0),
        ))
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

/// Greatest stamp in the log (the `ops_hlc` index makes this a lookup, not a scan). An
/// unreadable or unparsable stamp is an error: a silent zero would let the clock fall behind.
fn max_stored_hlc(conn: &Connection) -> Result<Hlc, Error> {
    let top: Option<String> = sql(conn.query_row("SELECT MAX(hlc) FROM ops", [], |r| r.get(0)))?;
    top.map_or_else(
        || Ok(Hlc::default()),
        |h| h.parse().map_err(|e| format!("stored stamp {h:?}: {e}")),
    )
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ops(
  circle INTEGER NOT NULL, dev INTEGER NOT NULL, seq INTEGER NOT NULL,
  hlc TEXT NOT NULL, obj TEXT NOT NULL, field TEXT NOT NULL, value TEXT NOT NULL, vv TEXT NOT NULL,
  PRIMARY KEY(circle, dev, seq)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS ops_obj ON ops(circle, obj);
CREATE INDEX IF NOT EXISTS ops_hlc ON ops(hlc);";

const SELECT_OPS: &str = "SELECT dev, seq, hlc, obj, field, value, vv FROM ops";

fn covered(w: &WireOp, by: &WireOp) -> bool {
    by.vv.get(&w.dev).is_some_and(|&s| s >= w.seq)
}

/// Revocation cuts of `circle`: per revoked author, how many of their own ops the revoker had
/// seen. With two cuts for one author (concurrent revokers) the stricter one applies.
fn cuts_on(conn: &Connection, circle: Circle) -> Cuts {
    let mut cuts = Cuts::new();
    let Ok(mut st) =
        conn.prepare_cached("SELECT field, value FROM ops WHERE circle = ?1 AND obj = ?2")
    else {
        return cuts;
    };
    let rows = st.query_map(params![db(circle), CIRCLE_OBJ], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    });
    for (field, value) in rows.into_iter().flatten().flatten() {
        if let (Some(member), Ok(cut)) = (parse_revoked(&field), value.parse::<u64>()) {
            cuts.entry(member)
                .and_modify(|c| *c = (*c).min(cut))
                .or_insert(cut);
        }
    }
    cuts
}

type Cuts = BTreeMap<Device, u64>;

/// One object's writes in one circle: `(circle, object, [(field, value)])`.
type Batch = (Circle, String, Vec<(String, String)>);

/// An op is shown unless its author was revoked and it is beyond the cut (S-09). The test is on
/// the author's own sequence number, so it is causal: no clock can move an op to the safe side.
fn kept(o: &WireOp, cuts: &Cuts) -> bool {
    cuts.get(&o.dev).is_none_or(|&cut| o.seq <= cut)
}

fn hlc_of(op: &WireOp) -> Hlc {
    op.hlc.parse().unwrap_or_default() // the stamp text is validated at ingest, see `import`
}

/// SQLite integers are `i64`; `usize`/`u64` are not bindable in rusqlite 0.42.
fn db(n: impl TryInto<i64>) -> i64 {
    n.try_into().unwrap_or(i64::MAX)
}

fn sql<T>(r: rusqlite::Result<T>) -> Result<T, Error> {
    r.map_err(|e| e.to_string())
}

/// What B shows of one object, and what it holds aside.
struct Derived {
    view: ObjView,
    conflicts: Vec<Conflict>,
    notice: Option<Notice>,
    quarantine: BTreeSet<Quarantined>,
}

/// Visible state of one object from all its ops. Pure: the only place B's rules live.
fn derive(obj: &str, ops: &[WireOp], cuts: &Cuts) -> Derived {
    // Ops beyond a revocation cut take no part: they do not win, conflict, cover or resurrect.
    let ops: Vec<&WireOp> = ops.iter().filter(|o| kept(o, cuts)).collect();
    let mut by_field: BTreeMap<&str, Vec<&WireOp>> = BTreeMap::new();
    for op in &ops {
        by_field.entry(op.field.as_str()).or_default().push(op);
    }
    let heads = |all: &[&WireOp]| -> Vec<(Hlc, WireOp)> {
        all.iter()
            .filter(|w| !all.iter().any(|x| covered(w, x)))
            .map(|w| (hlc_of(w), (*w).clone()))
            .collect()
    };
    let mut view = ObjView {
        deleted: false,
        moved: false,
        fields: BTreeMap::new(),
    };
    let mut conflicts = Vec::new();
    let mut notice = None;
    let mut quarantine = BTreeSet::new();
    for (field, all) in &by_field {
        let Some(((_, win), losers)) = pick(&heads(all)) else {
            continue;
        };
        if *field == DELETED {
            if win.value == "1" {
                let resurrected = ops
                    .iter()
                    .any(|w| w.field != DELETED && w.field != MOVED && !covered(w, &win));
                view.deleted = !resurrected;
                if resurrected {
                    notice = Some(Notice {
                        obj: obj.to_owned(),
                        deleter: win.dev,
                    });
                }
            }
        } else if *field == MOVED {
            // Whatever the move had not seen was made in the old circle by someone who did not
            // know; it is held aside for the mover, never put back (S-10).
            view.moved = true;
            for w in ops
                .iter()
                .filter(|w| w.field != DELETED && w.field != MOVED && !covered(w, &win))
            {
                quarantine.insert(Quarantined {
                    obj: obj.to_owned(),
                    author: w.dev,
                    reason: Reason::MovedAway,
                });
            }
        } else {
            view.fields.insert((*field).to_owned(), win.value);
            conflicts.extend(losers.into_iter().map(|(h, l)| Conflict {
                obj: obj.to_owned(),
                field: (*field).to_owned(),
                lost: l.value,
                hlc: h.to_string(),
            }));
        }
    }
    Derived {
        view,
        conflicts,
        notice,
        quarantine,
    }
}

fn row_to_op(r: &rusqlite::Row<'_>) -> rusqlite::Result<WireOp> {
    let vv: String = r.get(6)?;
    Ok(WireOp {
        dev: r.get(0)?,
        seq: u64::try_from(r.get::<_, i64>(1)?).unwrap_or(0),
        hlc: r.get(2)?,
        obj: r.get(3)?,
        field: r.get(4)?,
        value: r.get(5)?,
        vv: serde_json::from_str(&vv).unwrap_or_default(),
    })
}

impl AltB {
    /// File-backed replica (S-07 reopens it, S-12 opens it from several processes). WAL with a
    /// full sync per commit, and writers wait for each other instead of failing.
    pub fn open(device: Device, circles: &[Circle], path: &Path) -> Result<Self, Error> {
        // Switching to WAL on a database another process is creating at that very moment fails
        // with "locked" without waiting (S-12 found it), so the first open retries.
        let mut tries = 0;
        loop {
            let opened = sql(Connection::open(path)).and_then(|conn| {
                sql(conn.busy_timeout(std::time::Duration::from_secs(30)))?;
                sql(conn.pragma_update(None, "journal_mode", "WAL"))?;
                sql(conn.pragma_update(None, "synchronous", "FULL"))?;
                Self::with(device, circles, conn)
            });
            match opened {
                Err(e) if e.contains("locked") && tries < 100 => {
                    tries += 1;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                done => return done,
            }
        }
    }

    /// SQLite's own consistency check of the whole file.
    #[must_use]
    pub fn integrity_ok(&self) -> bool {
        self.conn
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .is_ok_and(|s| s == "ok")
    }

    fn with(device: Device, circles: &[Circle], conn: Connection) -> Result<Self, Error> {
        sql(conn.execute_batch(SCHEMA))?;
        // The clock is not stored on its own: it is the greatest stamp in the log, so it can
        // never fall behind what this store already holds, across restarts.
        let clock = max_stored_hlc(&conn)?;
        let mut me = Self {
            device,
            circles: circles.iter().copied().collect(),
            conn,
            clock,
            now: None,
            index: Index::new(),
            schema: Schema::V0,
        };
        me.rebuild_index();
        Ok(me)
    }

    fn ops_where(&self, filter: &str, args: impl rusqlite::Params) -> Vec<WireOp> {
        let Ok(mut st) = self
            .conn
            .prepare_cached(&format!("{SELECT_OPS} WHERE {filter}"))
        else {
            return vec![];
        };
        st.query_map(args, row_to_op)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    /// What this replica has of `circle`: highest sequence per author.
    #[must_use]
    pub fn vv(&self, circle: Circle) -> Vv {
        vv_on(&self.conn, circle)
    }

    /// Ops of `circle` the holder of `have` lacks, in author/sequence order.
    #[must_use]
    pub fn ops_after(&self, circle: Circle, have: &Vv) -> Vec<WireOp> {
        let mut ops = self.ops_where("circle = ?1 ORDER BY dev, seq", [db(circle)]);
        ops.retain(|o| o.seq > have.get(&o.dev).copied().unwrap_or(0));
        ops
    }

    /// Ops of one object that are shown: those of a revoked author beyond the cut are held
    /// aside, so an object that only they wrote does not exist yet.
    fn obj_ops(&self, circle: Circle, obj: &str) -> Vec<WireOp> {
        let cuts = cuts_on(&self.conn, circle);
        let mut ops = self.ops_where("circle = ?1 AND obj = ?2", params![db(circle), obj]);
        ops.retain(|o| kept(o, &cuts));
        ops
    }

    fn view_of(&self, circle: Circle, obj: &str) -> Result<Derived, Error> {
        let ops = self.obj_ops(circle, obj);
        if ops.is_empty() {
            return Err(format!("no such object {obj}"));
        }
        Ok(derive(obj, &ops, &Cuts::new()))
    }

    fn touch(&mut self, circle: Circle, obj: &str) {
        if obj == CIRCLE_OBJ {
            // A revocation changes what other objects show, not only itself.
            self.rebuild_index();
            return;
        }
        let view = derive(obj, &self.obj_ops(circle, obj), &Cuts::new()).view;
        match index_entry(&view) {
            Some(t) => self.index.insert((circle, obj.to_owned()), t),
            None => self.index.remove(&(circle, obj.to_owned())),
        };
    }

    fn insert(tx: &rusqlite::Transaction<'_>, circle: Circle, o: &WireOp) -> Result<usize, Error> {
        let vv = serde_json::to_string(&o.vv).map_err(|e| e.to_string())?;
        sql(tx.execute(
            "INSERT OR IGNORE INTO ops(circle, dev, seq, hlc, obj, field, value, vv)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                db(circle),
                o.dev,
                db(o.seq),
                o.hlc,
                o.obj,
                o.field,
                o.value,
                vv
            ],
        ))
    }

    fn import(&mut self, circle: Circle, ops: &[WireOp]) -> Result<bool, Error> {
        if !self.circles.contains(&circle) {
            return Err(format!(
                "circle {circle} is not held by device {}",
                self.device
            ));
        }
        if let Some(bad) = ops.iter().find(|o| o.hlc.parse::<Hlc>().is_err()) {
            return Err(format!("malformed stamp {:?}", bad.hlc));
        }
        // Skew is judged on what is new: an op already stored was accepted when it arrived.
        let held = self.vv(circle);
        let newest = ops
            .iter()
            .filter(|o| o.seq > held.get(&o.dev).copied().unwrap_or(0))
            .map(hlc_of)
            .max();
        if let (Some(h), Some(now)) = (newest, self.now)
            && h.skewed(now)
        {
            return Err(format!("stamp {h} is more than 60 s ahead of the clock"));
        }
        // Each author's log must arrive without holes: `vv` (and so every coverage decision)
        // assumes it. Ops we already have are fine; a gap is refused, never imported.
        let mut next = self.vv(circle);
        let mut sorted: Vec<&WireOp> = ops.iter().collect();
        sorted.sort_by_key(|o| (o.dev, o.seq));
        for o in sorted {
            let have = next.get(&o.dev).copied().unwrap_or(0);
            if o.seq > have {
                if o.seq != have + 1 {
                    return Err(format!(
                        "gap in the log of device {}: have {have}, got {}",
                        o.dev, o.seq
                    ));
                }
                next.insert(o.dev, o.seq);
            }
        }
        let tx = sql(self.conn.transaction())?;
        let mut added = 0;
        for o in ops {
            added += Self::insert(&tx, circle, o)?;
        }
        sql(tx.commit())?;
        if let Some(h) = newest {
            self.clock = self.clock.recv(h);
        }
        let touched: BTreeSet<&str> = ops.iter().map(|o| o.obj.as_str()).collect();
        for obj in touched {
            self.touch(circle, obj);
        }
        Ok(added > 0)
    }

    /// The writes one op makes, validated against what this replica shows. A move writes two
    /// circles; they commit together.
    fn plan(&self, circle: Circle, op: Op) -> Result<Vec<Batch>, Error> {
        Ok(match op {
            Op::Create { obj, fields } => {
                if fields.is_empty() || fields.contains_key(DELETED) || fields.contains_key(MOVED) {
                    return Err("create needs fields, and not a reserved register".into());
                }
                if !self.obj_ops(circle, &obj).is_empty() {
                    return Err(format!("object {obj} exists"));
                }
                vec![(circle, obj, fields.into_iter().collect())]
            }
            Op::Set { obj, field, value } => {
                if field == DELETED || field == MOVED {
                    return Err("the delete and move registers are not fields".into());
                }
                Self::live(&self.view_of(circle, &obj)?.view, &obj)?;
                vec![(circle, obj, vec![(field, value)])]
            }
            Op::Delete { obj } => {
                Self::live(&self.view_of(circle, &obj)?.view, &obj)?;
                vec![(circle, obj, vec![(DELETED.to_owned(), "1".to_owned())])]
            }
            Op::Restore { obj } => {
                let view = self.view_of(circle, &obj)?.view;
                if view.moved {
                    return Err(format!("object {obj} was moved"));
                }
                if !view.deleted {
                    return Err(format!("object {obj} is not deleted"));
                }
                vec![(circle, obj, vec![(DELETED.to_owned(), "0".to_owned())])]
            }
            Op::Revoke { member } => {
                if member == self.device {
                    return Err("a device cannot revoke itself".into());
                }
                if cuts_on(&self.conn, circle).contains_key(&member) {
                    return Err(format!("device {member} is already revoked"));
                }
                // ponytail: read outside the write transaction. A racing local process can only
                // make the cut lower, which is the stricter side.
                let cut = self.vv(circle).get(&member).copied().unwrap_or(0);
                vec![(
                    circle,
                    CIRCLE_OBJ.to_owned(),
                    vec![(revoked_field(member), cut.to_string())],
                )]
            }
            Op::Move { obj, to } => {
                if to == circle || !self.circles.contains(&to) {
                    return Err(format!(
                        "device {} cannot move an object from circle {circle} to {to}",
                        self.device
                    ));
                }
                let seen = self.view_of(circle, &obj)?;
                Self::live(&seen.view, &obj)?;
                if !seen.conflicts.is_empty() {
                    return Err(format!(
                        "object {obj} has open conflicts: resolve them before moving it"
                    ));
                }
                if !self.obj_ops(to, &obj).is_empty() {
                    return Err(format!("object {obj} exists in circle {to}"));
                }
                vec![
                    (to, obj.clone(), seen.view.fields.into_iter().collect()),
                    (circle, obj, vec![(MOVED.to_owned(), to.to_string())]),
                ]
            }
        })
    }

    /// A write needs an object that is neither deleted nor moved away.
    fn live(view: &ObjView, obj: &str) -> Result<(), Error> {
        if view.moved {
            return Err(format!("object {obj} was moved to another circle"));
        }
        if view.deleted {
            return Err(format!("object {obj} is deleted"));
        }
        Ok(())
    }

    /// S-07: a crash in the middle of an import. Inserts the first `apply` ops inside the
    /// import's transaction and abandons it uncommitted.
    ///
    /// `ponytail:` an abort, not a kill. The killed-process case is `tests/processes.rs`; a
    /// power cut is covered by neither.
    pub fn abandon_import(
        &mut self,
        circle: Circle,
        ops: &[WireOp],
        apply: usize,
    ) -> Result<(), Error> {
        let tx = sql(self.conn.transaction())?;
        for o in ops.iter().take(apply) {
            Self::insert(&tx, circle, o)?;
        }
        drop(tx); // rollback
        Ok(())
    }
}

impl Replica for AltB {
    type Sync = BSync;

    fn new(device: Device, circles: &[Circle]) -> Self {
        let conn = Connection::open_in_memory().unwrap_or_else(|e| panic!("sqlite: {e}"));
        Self::with(device, circles, conn).unwrap_or_else(|e| panic!("sqlite: {e}"))
    }

    fn set_schema(&mut self, schema: Schema) {
        self.schema = schema;
    }

    fn device(&self) -> Device {
        self.device
    }

    fn set_now(&mut self, now_ms: u64) {
        self.now = Some(now_ms);
    }

    fn apply_local(&mut self, circle: Circle, op: Op, now_ms: u64) -> Result<(), Error> {
        if !self.circles.contains(&circle) {
            return Err(format!(
                "circle {circle} is not held by device {}",
                self.device
            ));
        }
        match &op {
            Op::Create { fields, .. } => {
                for f in fields.keys() {
                    self.schema.check_write(f)?;
                }
            }
            Op::Set { field, .. } => self.schema.check_write(field)?,
            _ => {}
        }
        let batches = self.plan(circle, op)?;
        // IMMEDIATE takes the write lock before the sequence number is read: two processes of
        // one device (S-12) can never choose the same `seq` or the same stamp.
        let tx = sql(self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate))?;
        let last_own = max_stored_hlc(&tx)?;
        self.clock = self.clock.recv(last_own);
        for (c, obj, writes) in &batches {
            let mut vv = vv_on(&tx, *c);
            for (field, value) in writes {
                let seq = vv.get(&self.device).copied().unwrap_or(0) + 1;
                self.clock = self.clock.tick(now_ms, self.device);
                let o = WireOp {
                    dev: self.device,
                    seq,
                    hlc: self.clock.to_string(),
                    obj: obj.clone(),
                    field: field.clone(),
                    value: value.clone(),
                    vv: vv.clone(),
                };
                // `INSERT OR IGNORE` would hide a collision (e.g. a failed `vv` read reusing a
                // sequence number) and drop the user's write while reporting success.
                if Self::insert(&tx, *c, &o)? != 1 {
                    return Err(format!("op ({}, {seq}) was not stored", self.device));
                }
                vv.insert(self.device, seq);
            }
        }
        sql(tx.commit())?;
        for (c, obj, _) in &batches {
            self.touch(*c, obj);
        }
        Ok(())
    }

    fn generate(&mut self, circle: Circle, st: &mut BSync) -> Option<Vec<u8>> {
        if !self.circles.contains(&circle) {
            return None;
        }
        if !st.sent_have {
            st.sent_have = true;
            return serde_json::to_vec(&Msg::Have(self.vv(circle))).ok();
        }
        if !st.sent_ops {
            let peer = st.peer_vv.as_ref()?;
            st.sent_ops = true;
            let ops = self.ops_after(circle, peer);
            if !ops.is_empty() {
                return serde_json::to_vec(&Msg::Ops(ops)).ok();
            }
        }
        None
    }

    fn receive(&mut self, circle: Circle, st: &mut BSync, msg: &[u8]) -> Result<bool, Error> {
        if !self.circles.contains(&circle) {
            return Err(format!(
                "circle {circle} is not held by device {}",
                self.device
            ));
        }
        match serde_json::from_slice::<Msg>(msg).map_err(|e| e.to_string())? {
            Msg::Have(vv) => {
                st.peer_vv = Some(vv);
                Ok(false)
            }
            Msg::Ops(ops) => self.import(circle, &ops),
        }
    }

    fn upload(&mut self, circle: Circle) -> Vec<u8> {
        let ops = self.ops_after(circle, &Vv::new());
        serde_json::to_vec(&Msg::Ops(ops)).unwrap_or_default()
    }

    fn accept_upload(&mut self, circle: Circle, msg: &[u8]) -> Result<bool, Error> {
        match serde_json::from_slice::<Msg>(msg).map_err(|e| e.to_string())? {
            Msg::Ops(ops) => self.import(circle, &ops),
            Msg::Have(_) => Err("an upload carries ops, not a version vector".into()),
        }
    }

    fn snapshot(&mut self, circle: Circle) -> Snapshot {
        let cuts = cuts_on(&self.conn, circle);
        let mut by_obj: BTreeMap<String, Vec<WireOp>> = BTreeMap::new();
        let mut snap = Snapshot::default();
        for o in self.ops_where("circle = ?1", [db(circle)]) {
            if o.obj == CIRCLE_OBJ {
                continue;
            }
            if !kept(&o, &cuts) {
                snap.quarantine.insert(Quarantined {
                    obj: o.obj.clone(),
                    author: o.dev,
                    reason: Reason::Revoked,
                });
            }
            by_obj.entry(o.obj.clone()).or_default().push(o);
        }
        for (obj, ops) in by_obj {
            if ops.iter().all(|o| !kept(o, &cuts)) {
                continue; // only a revoked author's unseen work: the object does not exist yet
            }
            let d = derive(&obj, &ops, &cuts);
            snap.quarantine.extend(d.quarantine);
            if d.view.moved {
                continue;
            }
            snap.conflicts.extend(d.conflicts);
            snap.notices.extend(d.notice);
            snap.objects.insert(obj, d.view);
        }
        self.schema.mask(&mut snap);
        snap
    }

    fn index(&self) -> Index {
        self.index.clone()
    }

    fn rebuild_index(&mut self) {
        let mut fresh = Index::new();
        for c in self.circles.clone() {
            for (obj, view) in self.snapshot(c).objects {
                if let Some(t) = index_entry(&view) {
                    fresh.insert((c, obj), t);
                }
            }
        }
        self.index = fresh;
    }

    fn storage_bytes(&mut self) -> usize {
        let get = |pragma: &str| -> i64 {
            self.conn
                .query_row(&format!("PRAGMA {pragma}"), [], |r| r.get(0))
                .unwrap_or(0)
        };
        usize::try_from(get("page_count") * get("page_size")).unwrap_or(0)
    }

    fn dump_text(&mut self) -> String {
        let mut out = String::new();
        // Every row, not only held circles: a stray row of another circle is exactly the leak.
        for o in self.ops_where("1 = 1", params![]) {
            let _ = writeln!(out, "{o:?}");
        }
        for c in self.circles.clone() {
            let _ = writeln!(out, "{:?}", self.snapshot(c));
        }
        let _ = writeln!(out, "{:?}", self.index);
        out
    }
}
