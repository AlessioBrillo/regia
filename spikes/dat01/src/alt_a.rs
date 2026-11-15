//! Alternative A: one Automerge document per circle. The document is the source of truth; the
//! local index is a projection. Sync is Automerge's own protocol.
//!
//! Layout: `ROOT[object id]` is a map of field to `"<hlc>\u{1}<value>"`. Concurrent puts to one
//! field are Automerge's native conflicts, so the greatest [`Hlc`] among `get_all` wins (the same
//! lww rule as B). Deleting is *not* removing the key (a concurrent edit would then be lost
//! silently): it writes the delete register with the heads the deleter had seen. An edit not in
//! the history of those heads resurrects the object (S-05). Each commit's message is
//! `"<object>\t<field>"`, which is how the changes unseen by a delete are attributed to objects.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

use automerge::{
    ActorId, AutoCommit, ChangeHash, ObjType, ROOT, ReadDoc,
    sync::{self, SyncDoc},
    transaction::{CommitOptions, Transactable},
};

use crate::{
    CIRCLE_OBJ, Circle, Conflict, DELETED, Device, Error, Hlc, Index, MOVED, Notice, ObjView, Op,
    Quarantined, Reason, Replica, Snapshot, index_entry, parse_revoked, pick, revoked_field,
};

const SEP: char = '\u{1}';

/// Session state for one (peer, circle).
pub struct ASync(sync::State);

impl Default for ASync {
    fn default() -> Self {
        Self(sync::State::new())
    }
}

/// Replica holding one document per circle.
pub struct AltA {
    device: Device,
    docs: BTreeMap<Circle, AutoCommit>,
    clock: Hlc,
    now: Option<u64>,
    index: Index,
}

#[allow(clippy::needless_pass_by_value)] // used as `.map_err(err)`, which passes the error by value
fn err<E: ToString>(e: E) -> Error {
    e.to_string()
}

fn actor(device: Device) -> ActorId {
    ActorId::from(device.to_be_bytes().to_vec())
}

fn encode(h: Hlc, v: &str) -> String {
    format!("{h}{SEP}{v}")
}

fn decode(s: &str) -> Option<(Hlc, &str)> {
    let (h, v) = s.split_once(SEP)?;
    Some((h.parse().ok()?, v))
}

/// What A shows of one object, and what it holds aside.
struct Derived {
    view: ObjView,
    conflicts: Vec<Conflict>,
    notice: Option<Notice>,
    quarantine: BTreeSet<Quarantined>,
}

/// Devices whose changes, not seen by `seen` (a comma list of change hashes), wrote a field of
/// `obj` other than a register. Hashes the document does not hold are ignored.
fn unseen_writers(doc: &mut AutoCommit, obj: &str, seen: &str) -> Vec<Device> {
    let parsed: Vec<ChangeHash> = seen.split(',').filter_map(|h| h.parse().ok()).collect();
    let seen: Vec<ChangeHash> = parsed
        .into_iter()
        .filter(|h| doc.get_change_by_hash(h).is_some())
        .collect();
    doc.get_changes(&seen)
        .iter()
        .filter_map(|c| {
            let (o, f) = c.message()?.split_once('\t')?;
            (o == obj && f != DELETED && f != MOVED).then(|| device_of(c.actor_id()))?
        })
        .collect()
}

/// Visible state of one object, or `None` if the document has no such object.
fn derive(doc: &mut AutoCommit, obj: &str) -> Option<Derived> {
    let (_, id) = doc.get(ROOT, obj).ok()??;
    let keys: Vec<String> = doc.keys(&id).collect();
    let mut view = ObjView {
        deleted: false,
        moved: false,
        fields: BTreeMap::new(),
    };
    let mut conflicts = Vec::new();
    let mut notice = None;
    let mut quarantine = BTreeSet::new();
    for key in keys {
        let heads: Vec<(Hlc, String)> = doc
            .get_all(&id, key.as_str())
            .ok()?
            .into_iter()
            .filter_map(|(v, _)| decode(v.as_str()?).map(|(h, v)| (h, v.to_owned())))
            .collect();
        let Some(((win_hlc, win), losers)) = pick(&heads) else {
            continue;
        };
        if key == DELETED {
            if let Some(seen) = win.strip_prefix("1|") {
                let resurrected = !unseen_writers(doc, obj, seen).is_empty();
                view.deleted = !resurrected;
                if resurrected {
                    notice = Some(Notice {
                        obj: obj.to_owned(),
                        deleter: win_hlc.device,
                    });
                }
            }
        } else if key == MOVED {
            // What the move had not seen was made in the old circle by someone who did not know;
            // it is held aside for the mover, never put back (S-10).
            view.moved = true;
            let seen = win.split_once('|').map_or("", |(_, seen)| seen);
            for author in unseen_writers(doc, obj, seen) {
                quarantine.insert(Quarantined {
                    obj: obj.to_owned(),
                    author,
                    reason: Reason::MovedAway,
                });
            }
        } else {
            view.fields.insert(key.clone(), win);
            conflicts.extend(losers.into_iter().map(|(h, lost)| Conflict {
                obj: obj.to_owned(),
                field: key.clone(),
                lost,
                hlc: h.to_string(),
            }));
        }
    }
    Some(Derived {
        view,
        conflicts,
        notice,
        quarantine,
    })
}

/// Author of a change: the simulator derives every actor id from the device (`actor`).
fn device_of(a: &ActorId) -> Option<Device> {
    Some(Device::from_be_bytes(a.to_bytes().try_into().ok()?))
}

/// Revocation cuts of a document: per revoked author, how many of their own changes the revoker
/// had seen. With two cuts for one author the stricter one applies.
fn cuts_of(doc: &mut AutoCommit) -> BTreeMap<Device, u64> {
    let mut cuts = BTreeMap::new();
    let Some((_, id)) = doc.get(ROOT, CIRCLE_OBJ).ok().flatten() else {
        return cuts;
    };
    let keys: Vec<String> = doc.keys(&id).collect();
    for key in keys {
        let Some(member) = parse_revoked(&key) else {
            continue;
        };
        for (v, _) in doc.get_all(&id, key.as_str()).unwrap_or_default() {
            if let Some(cut) = v
                .as_str()
                .and_then(decode)
                .and_then(|(_, c)| c.parse::<u64>().ok())
            {
                cuts.entry(member)
                    .and_modify(|c: &mut u64| *c = (*c).min(cut))
                    .or_insert(cut);
            }
        }
    }
    cuts
}

/// The document as the circle shows it once authors were revoked: without their changes beyond
/// the cut (S-09), and without whatever was built on those, because Automerge cannot take a
/// change out of a history that depends on it. `None` when nobody was revoked.
///
/// The raw document keeps everything, so nothing is lost; only this replay is shown.
fn taint(doc: &mut AutoCommit) -> Option<(AutoCommit, BTreeSet<Quarantined>)> {
    let cuts = cuts_of(doc);
    if cuts.is_empty() {
        return None;
    }
    let (mut bad, mut quarantined, mut keep) = (BTreeSet::new(), BTreeSet::new(), Vec::new());
    for c in doc.get_changes(&[]) {
        let author = device_of(c.actor_id());
        let direct = author
            .and_then(|d| cuts.get(&d))
            .is_some_and(|&cut| c.seq() > cut);
        let dependent = c.deps().iter().any(|h| bad.contains(h));
        if direct || dependent {
            bad.insert(c.hash());
            if let Some((obj, _)) = c.message().and_then(|m| m.split_once('\t'))
                && obj != CIRCLE_OBJ
            {
                quarantined.insert(Quarantined {
                    obj: obj.to_owned(),
                    author: author.unwrap_or(Device::MAX),
                    reason: if direct {
                        Reason::Revoked
                    } else {
                        Reason::Dependent
                    },
                });
            }
        } else {
            keep.push(c.clone());
        }
    }
    let mut visible = AutoCommit::new();
    visible
        .apply_changes(keep)
        .unwrap_or_else(|e| panic!("automerge replay: {e}"));
    Some((visible, quarantined))
}

/// [`derive`] on what the circle shows (see [`taint`]).
fn derive_vis(doc: &mut AutoCommit, obj: &str) -> Option<Derived> {
    let mut visible = taint(doc).map(|(d, _)| d);
    derive(visible.as_mut().unwrap_or(doc), obj)
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

/// Greatest stamp stored in the given objects (all of them if `objs` is `None`).
fn max_stamp(doc: &mut AutoCommit, objs: Option<&BTreeSet<String>>) -> Option<Hlc> {
    let objs: Vec<String> =
        objs.map_or_else(|| doc.keys(ROOT).collect(), |o| o.iter().cloned().collect());
    let mut max = None;
    for obj in objs {
        let Some((_, id)) = doc.get(ROOT, obj.as_str()).ok().flatten() else {
            continue;
        };
        let keys: Vec<String> = doc.keys(&id).collect();
        for key in keys {
            for (v, _) in doc.get_all(&id, key.as_str()).unwrap_or_default() {
                max = max.max(v.as_str().and_then(decode).map(|(h, _)| h));
            }
        }
    }
    max
}

/// Objects named by the commit messages of the changes after `heads`.
fn changed_objs(doc: &mut AutoCommit, heads: &[ChangeHash]) -> BTreeSet<String> {
    doc.get_changes(heads)
        .iter()
        .filter_map(|c| Some(c.message()?.split_once('\t')?.0.to_owned()))
        .collect()
}

impl AltA {
    fn doc(&mut self, circle: Circle) -> Result<&mut AutoCommit, Error> {
        self.docs
            .get_mut(&circle)
            .ok_or_else(|| format!("circle {circle} is not held by device {}", self.device))
    }

    fn view_of(&mut self, circle: Circle, obj: &str) -> Result<Derived, Error> {
        let doc = self.doc(circle)?;
        derive_vis(doc, obj).ok_or_else(|| format!("no such object {obj}"))
    }

    fn touch(&mut self, circle: Circle, obj: &str) {
        if obj == CIRCLE_OBJ {
            // A revocation changes what other objects show, not only itself.
            self.rebuild_index();
            return;
        }
        let view = self
            .docs
            .get_mut(&circle)
            .and_then(|d| derive_vis(d, obj))
            .map(|d| d.view);
        match view.as_ref().and_then(index_entry) {
            Some(t) => self.index.insert((circle, obj.to_owned()), t),
            None => self.index.remove(&(circle, obj.to_owned())),
        };
    }

    fn put_stamped(
        &mut self,
        circle: Circle,
        obj: &str,
        writes: &[(String, String)],
        now_ms: u64,
        field_for_message: &str,
    ) -> Result<(), Error> {
        let (mut clock, device) = (self.clock, self.device);
        let doc = self.doc(circle)?;
        let (_, id) = doc
            .get(ROOT, obj)
            .map_err(err)?
            .ok_or_else(|| format!("no such object {obj}"))?;
        for (field, value) in writes {
            clock = clock.tick(now_ms, device);
            doc.put(&id, field.as_str(), encode(clock, value))
                .map_err(err)?;
        }
        doc.commit_with(
            CommitOptions::default().with_message(format!("{obj}\t{field_for_message}")),
        );
        self.clock = clock;
        Ok(())
    }

    fn create_obj(
        &mut self,
        circle: Circle,
        obj: &str,
        fields: BTreeMap<String, String>,
        now_ms: u64,
    ) -> Result<(), Error> {
        let doc = self.doc(circle)?;
        if doc.get(ROOT, obj).map_err(err)?.is_some() {
            return Err(format!("object {obj} exists"));
        }
        doc.put_object(ROOT, obj, ObjType::Map).map_err(err)?;
        let writes: Vec<_> = fields.into_iter().collect();
        self.put_stamped(circle, obj, &writes, now_ms, "*")
    }

    fn heads_text(&mut self, circle: Circle) -> Result<String, Error> {
        Ok(self
            .doc(circle)?
            .get_heads()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","))
    }

    /// Merge into a circle's document whatever `apply` brings, after the skew check: Automerge
    /// applies as it receives, so `apply` is tried on a fork first (`true` = that trial run).
    fn merge(
        &mut self,
        circle: Circle,
        mut apply: impl FnMut(&mut AutoCommit, bool) -> Result<(), Error>,
    ) -> Result<bool, Error> {
        let now = self.now;
        let doc = self.doc(circle)?;
        let before = doc.get_heads();
        if let Some(now) = now {
            let mut trial = doc.fork();
            apply(&mut trial, true)?;
            let objs = changed_objs(&mut trial, &before);
            // No parseable commit message: scan everything rather than skip the check.
            let scope = (!objs.is_empty()).then_some(&objs);
            if let Some(h) = max_stamp(&mut trial, scope)
                && h.skewed(now)
            {
                return Err(format!("stamp {h} is more than 60 s ahead of the clock"));
            }
        }
        apply(doc, false)?;
        if doc.get_heads() == before {
            return Ok(false);
        }
        let objs = changed_objs(doc, &before);
        if let Some(h) = max_stamp(doc, (!objs.is_empty()).then_some(&objs)) {
            self.clock = self.clock.recv(h);
        }
        for obj in objs {
            self.touch(circle, &obj);
        }
        Ok(true)
    }

    /// The document as bytes, as a device would persist it.
    pub fn save_circle(&mut self, circle: Circle) -> Result<Vec<u8>, Error> {
        Ok(self.doc(circle)?.save())
    }

    /// Replaces a circle's document by one loaded from `bytes` (a restart after a crash). Uses a
    /// fresh actor id: reusing the old one after losing local changes would reissue operation ids.
    pub fn load_circle(&mut self, circle: Circle, bytes: &[u8]) -> Result<(), Error> {
        let mut doc = AutoCommit::load(bytes).map_err(err)?;
        doc.set_actor(ActorId::random());
        // The clock is the greatest stamp in the data, so a restart cannot put it behind.
        self.clock = self
            .clock
            .recv(max_stamp(&mut doc, None).unwrap_or_default());
        self.docs.insert(circle, doc);
        self.rebuild_index();
        Ok(())
    }
}

impl Replica for AltA {
    type Sync = ASync;

    fn new(device: Device, circles: &[Circle]) -> Self {
        let docs = circles
            .iter()
            .map(|&c| (c, AutoCommit::new().with_actor(actor(device))))
            .collect();
        Self {
            device,
            docs,
            clock: Hlc::default(),
            now: None,
            index: Index::new(),
        }
    }

    fn device(&self) -> Device {
        self.device
    }

    fn set_now(&mut self, now_ms: u64) {
        self.now = Some(now_ms);
    }

    fn apply_local(&mut self, circle: Circle, op: Op, now_ms: u64) -> Result<(), Error> {
        match op {
            Op::Create { obj, fields } => {
                if fields.is_empty() || fields.contains_key(DELETED) || fields.contains_key(MOVED) {
                    return Err("create needs fields, and not a reserved register".into());
                }
                self.create_obj(circle, &obj, fields, now_ms)?;
                self.touch(circle, &obj);
            }
            Op::Set { obj, field, value } => {
                if field == DELETED || field == MOVED {
                    return Err("the delete and move registers are not fields".into());
                }
                live(&self.view_of(circle, &obj)?.view, &obj)?;
                self.put_stamped(circle, &obj, &[(field.clone(), value)], now_ms, &field)?;
                self.touch(circle, &obj);
            }
            Op::Delete { obj } => {
                live(&self.view_of(circle, &obj)?.view, &obj)?;
                let seen = self.heads_text(circle)?;
                let write = [(DELETED.to_owned(), format!("1|{seen}"))];
                self.put_stamped(circle, &obj, &write, now_ms, DELETED)?;
                self.touch(circle, &obj);
            }
            Op::Restore { obj } => {
                let view = self.view_of(circle, &obj)?.view;
                if view.moved {
                    return Err(format!("object {obj} was moved"));
                }
                if !view.deleted {
                    return Err(format!("object {obj} is not deleted"));
                }
                let write = [(DELETED.to_owned(), "0".to_owned())];
                self.put_stamped(circle, &obj, &write, now_ms, DELETED)?;
                self.touch(circle, &obj);
            }
            Op::Revoke { member } => {
                if member == self.device {
                    return Err("a device cannot revoke itself".into());
                }
                let doc = self.doc(circle)?;
                if cuts_of(doc).contains_key(&member) {
                    return Err(format!("device {member} is already revoked"));
                }
                // How many of the member's own changes this replica holds.
                let cut = doc
                    .get_changes(&[])
                    .iter()
                    .filter(|c| device_of(c.actor_id()) == Some(member))
                    .map(automerge::Change::seq)
                    .max()
                    .unwrap_or(0);
                if doc.get(ROOT, CIRCLE_OBJ).map_err(err)?.is_none() {
                    doc.put_object(ROOT, CIRCLE_OBJ, ObjType::Map)
                        .map_err(err)?;
                }
                let field = revoked_field(member);
                let write = [(field.clone(), cut.to_string())];
                self.put_stamped(circle, CIRCLE_OBJ, &write, now_ms, &field)?;
                self.touch(circle, CIRCLE_OBJ);
            }
            Op::Move { obj, to } => {
                if to == circle || !self.docs.contains_key(&to) {
                    return Err(format!(
                        "device {} cannot move an object from circle {circle} to {to}",
                        self.device
                    ));
                }
                let seen = self.view_of(circle, &obj)?;
                live(&seen.view, &obj)?;
                if !seen.conflicts.is_empty() {
                    return Err(format!(
                        "object {obj} has open conflicts: resolve them before moving it"
                    ));
                }
                // ponytail: two documents, two commits. A crash between them leaves the object in
                // both circles; B commits both in one transaction.
                self.create_obj(to, &obj, seen.view.fields, now_ms)?;
                let heads = self.heads_text(circle)?;
                let write = [(MOVED.to_owned(), format!("{to}|{heads}"))];
                self.put_stamped(circle, &obj, &write, now_ms, MOVED)?;
                self.touch(to, &obj);
                self.touch(circle, &obj);
            }
        }
        Ok(())
    }

    fn generate(&mut self, circle: Circle, st: &mut ASync) -> Option<Vec<u8>> {
        let doc = self.docs.get_mut(&circle)?;
        doc.sync()
            .generate_sync_message(&mut st.0)
            .map(sync::Message::encode)
    }

    fn receive(&mut self, circle: Circle, st: &mut ASync, msg: &[u8]) -> Result<bool, Error> {
        let message = sync::Message::decode(msg).map_err(err)?;
        self.merge(circle, |doc, trial| {
            // The trial run gets a copy of the session state so the real one is not advanced twice.
            let mut copy = st.0.clone();
            let state = if trial { &mut copy } else { &mut st.0 };
            doc.sync()
                .receive_sync_message(state, message.clone())
                .map_err(err)
        })
    }

    fn upload(&mut self, circle: Circle) -> Vec<u8> {
        self.docs
            .get_mut(&circle)
            .map(AutoCommit::save)
            .unwrap_or_default()
    }

    fn accept_upload(&mut self, circle: Circle, msg: &[u8]) -> Result<bool, Error> {
        self.merge(circle, |doc, _| {
            doc.load_incremental(msg).map(|_| ()).map_err(err)
        })
    }

    fn snapshot(&mut self, circle: Circle) -> Snapshot {
        let mut snap = Snapshot::default();
        let Some(doc) = self.docs.get_mut(&circle) else {
            return snap;
        };
        let mut visible = taint(doc).map(|(d, held)| {
            snap.quarantine = held;
            d
        });
        let doc = visible.as_mut().unwrap_or(doc);
        let objs: Vec<String> = doc.keys(ROOT).collect();
        for obj in objs.into_iter().filter(|o| o != CIRCLE_OBJ) {
            if let Some(d) = derive(doc, &obj) {
                snap.quarantine.extend(d.quarantine);
                if d.view.moved {
                    continue;
                }
                snap.conflicts.extend(d.conflicts);
                snap.notices.extend(d.notice);
                snap.objects.insert(obj, d.view);
            }
        }
        snap
    }

    fn index(&self) -> Index {
        self.index.clone()
    }

    fn rebuild_index(&mut self) {
        let mut fresh = Index::new();
        let circles: Vec<Circle> = self.docs.keys().copied().collect();
        for c in circles {
            for (obj, view) in self.snapshot(c).objects {
                if let Some(t) = index_entry(&view) {
                    fresh.insert((c, obj), t);
                }
            }
        }
        self.index = fresh;
    }

    fn storage_bytes(&mut self) -> usize {
        self.docs.values_mut().map(|d| d.save().len()).sum()
    }

    fn dump_text(&mut self) -> String {
        let mut out = String::new();
        let circles: Vec<Circle> = self.docs.keys().copied().collect();
        for c in circles {
            if let Some(doc) = self.docs.get_mut(&c) {
                // Uncompressed on purpose: a compressed save could hide a leaked string.
                out.push_str(&String::from_utf8_lossy(&doc.save_nocompress()));
                out.push('\n');
            }
            let _ = writeln!(out, "{:?}", self.snapshot(c));
        }
        let _ = writeln!(out, "{:?}", self.index);
        out
    }
}
