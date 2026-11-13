//! Alternative B: SQLite is the source of truth. Every write is an immutable op carrying the
//! version vector its author had seen; visible state is derived from the ops. Sync sends the ops
//! the peer lacks, found by comparing version vectors (explicit operation log, custom sync).

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::{
    Circle, Conflict, DELETED, Device, Error, Hlc, Index, Notice, ObjView, Op, Replica, Snapshot,
    index_entry, pick,
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
    index: Index,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ops(
  circle INTEGER NOT NULL, dev INTEGER NOT NULL, seq INTEGER NOT NULL,
  hlc TEXT NOT NULL, obj TEXT NOT NULL, field TEXT NOT NULL, value TEXT NOT NULL, vv TEXT NOT NULL,
  PRIMARY KEY(circle, dev, seq)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS ops_obj ON ops(circle, obj);";

const SELECT_OPS: &str = "SELECT dev, seq, hlc, obj, field, value, vv FROM ops";

fn covered(w: &WireOp, by: &WireOp) -> bool {
    by.vv.get(&w.dev).is_some_and(|&s| s >= w.seq)
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

/// Visible state of one object from all its ops. Pure: the only place B's rules live.
fn derive(obj: &str, ops: &[WireOp]) -> (ObjView, Vec<Conflict>, Option<Notice>) {
    let mut by_field: BTreeMap<&str, Vec<&WireOp>> = BTreeMap::new();
    for op in ops {
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
        fields: BTreeMap::new(),
    };
    let mut conflicts = Vec::new();
    let mut notice = None;
    for (field, all) in &by_field {
        let Some(((_, win), losers)) = pick(&heads(all)) else {
            continue;
        };
        if *field == DELETED {
            if win.value == "1" {
                let resurrected = ops.iter().any(|w| w.field != DELETED && !covered(w, &win));
                view.deleted = !resurrected;
                if resurrected {
                    notice = Some(Notice {
                        obj: obj.to_owned(),
                        deleter: win.dev,
                    });
                }
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
    (view, conflicts, notice)
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
    /// File-backed replica (S-07 reopens it).
    pub fn open(device: Device, circles: &[Circle], path: &Path) -> Result<Self, Error> {
        Self::with(device, circles, sql(Connection::open(path))?)
    }

    fn with(device: Device, circles: &[Circle], conn: Connection) -> Result<Self, Error> {
        sql(conn.execute_batch(SCHEMA))?;
        let mut me = Self {
            device,
            circles: circles.iter().copied().collect(),
            conn,
            clock: Hlc::default(),
            index: Index::new(),
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
        let Ok(mut st) = self
            .conn
            .prepare_cached("SELECT dev, MAX(seq) FROM ops WHERE circle = ?1 GROUP BY dev")
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

    /// Ops of `circle` the holder of `have` lacks, in author/sequence order.
    #[must_use]
    pub fn ops_after(&self, circle: Circle, have: &Vv) -> Vec<WireOp> {
        let mut ops = self.ops_where("circle = ?1 ORDER BY dev, seq", [db(circle)]);
        ops.retain(|o| o.seq > have.get(&o.dev).copied().unwrap_or(0));
        ops
    }

    fn obj_ops(&self, circle: Circle, obj: &str) -> Vec<WireOp> {
        self.ops_where("circle = ?1 AND obj = ?2", params![db(circle), obj])
    }

    fn view_of(&self, circle: Circle, obj: &str) -> Result<ObjView, Error> {
        let ops = self.obj_ops(circle, obj);
        if ops.is_empty() {
            return Err(format!("no such object {obj}"));
        }
        Ok(derive(obj, &ops).0)
    }

    fn touch(&mut self, circle: Circle, obj: &str) {
        let (view, ..) = derive(obj, &self.obj_ops(circle, obj));
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
        let tx = sql(self.conn.transaction())?;
        let mut added = 0;
        for o in ops {
            added += Self::insert(&tx, circle, o)?;
        }
        sql(tx.commit())?;
        let touched: BTreeSet<&str> = ops.iter().map(|o| o.obj.as_str()).collect();
        for obj in touched {
            self.touch(circle, obj);
        }
        Ok(added > 0)
    }

    /// S-07: a crash in the middle of an import. Inserts the first `apply` ops inside the
    /// import's transaction and abandons it uncommitted.
    ///
    /// `ponytail:` an abort, not a power cut. A killed process is covered by SQLite's journal,
    /// which this spike does not re-test; add a child-process kill test if ADR-002 picks SQLite.
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

    fn device(&self) -> Device {
        self.device
    }

    fn apply_local(&mut self, circle: Circle, op: Op, now_ms: u64) -> Result<(), Error> {
        if !self.circles.contains(&circle) {
            return Err(format!(
                "circle {circle} is not held by device {}",
                self.device
            ));
        }
        let (obj, writes) = match op {
            Op::Create { obj, fields } => {
                if fields.is_empty() || fields.contains_key(DELETED) {
                    return Err("create needs fields, and not the delete register".into());
                }
                if !self.obj_ops(circle, &obj).is_empty() {
                    return Err(format!("object {obj} exists"));
                }
                (obj, fields.into_iter().collect::<Vec<_>>())
            }
            Op::Set { obj, field, value } => {
                if field == DELETED {
                    return Err("the delete register is not a field".into());
                }
                if self.view_of(circle, &obj)?.deleted {
                    return Err(format!("object {obj} is deleted"));
                }
                (obj, vec![(field, value)])
            }
            Op::Delete { obj } => {
                if self.view_of(circle, &obj)?.deleted {
                    return Err(format!("object {obj} is already deleted"));
                }
                (obj, vec![(DELETED.to_owned(), "1".to_owned())])
            }
            Op::Restore { obj } => {
                if !self.view_of(circle, &obj)?.deleted {
                    return Err(format!("object {obj} is not deleted"));
                }
                (obj, vec![(DELETED.to_owned(), "0".to_owned())])
            }
        };
        let mut vv = self.vv(circle);
        let tx = sql(self.conn.transaction())?;
        for (field, value) in writes {
            let seq = vv.get(&self.device).copied().unwrap_or(0) + 1;
            self.clock = self.clock.tick(now_ms, self.device);
            let o = WireOp {
                dev: self.device,
                seq,
                hlc: self.clock.to_string(),
                obj: obj.clone(),
                field,
                value,
                vv: vv.clone(),
            };
            Self::insert(&tx, circle, &o)?;
            vv.insert(self.device, seq);
        }
        sql(tx.commit())?;
        self.touch(circle, &obj);
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
        match serde_json::from_slice::<Msg>(msg).map_err(|e| e.to_string())? {
            Msg::Have(vv) => {
                st.peer_vv = Some(vv);
                Ok(false)
            }
            Msg::Ops(ops) => self.import(circle, &ops),
        }
    }

    fn snapshot(&mut self, circle: Circle) -> Snapshot {
        let mut by_obj: BTreeMap<String, Vec<WireOp>> = BTreeMap::new();
        for o in self.ops_where("circle = ?1", [db(circle)]) {
            by_obj.entry(o.obj.clone()).or_default().push(o);
        }
        let mut snap = Snapshot::default();
        for (obj, ops) in by_obj {
            let (view, conflicts, notice) = derive(&obj, &ops);
            snap.conflicts.extend(conflicts);
            snap.notices.extend(notice);
            snap.objects.insert(obj, view);
        }
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
            out.push_str(&format!("{o:?}\n"));
        }
        for c in self.circles.clone() {
            out.push_str(&format!("{:?}\n", self.snapshot(c)));
        }
        out.push_str(&format!("{:?}\n", self.index));
        out
    }
}
