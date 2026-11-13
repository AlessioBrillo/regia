//! Alternative A: one Automerge document per circle. The document is the source of truth; the
//! local index is a projection. Sync is Automerge's own protocol.
//!
//! Layout: `ROOT[object id]` is a map of field to `"<hlc>\u{1}<value>"`. Concurrent puts to one
//! field are Automerge's native conflicts, so the greatest [`Hlc`] among `get_all` wins (the same
//! lww rule as B). Deleting is *not* removing the key (a concurrent edit would then be lost
//! silently): it writes the delete register with the heads the deleter had seen. An edit not in
//! the history of those heads resurrects the object (S-05). Each commit's message is
//! `"<object>\t<field>"`, which is how the changes unseen by a delete are attributed to objects.

use std::collections::{BTreeMap, BTreeSet};

use automerge::{
    ActorId, AutoCommit, ChangeHash, ObjType, ROOT, ReadDoc,
    sync::{self, SyncDoc},
    transaction::{CommitOptions, Transactable},
};

use crate::{
    Circle, Conflict, DELETED, Device, Error, Hlc, Index, Notice, ObjView, Op, Replica, Snapshot,
    index_entry, pick,
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
    index: Index,
}

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

/// Visible state of one object, or `None` if the document has no such object.
fn derive(doc: &mut AutoCommit, obj: &str) -> Option<(ObjView, Vec<Conflict>, Option<Notice>)> {
    let (_, id) = doc.get(ROOT, obj).ok()??;
    let keys: Vec<String> = doc.keys(&id).collect();
    let mut view = ObjView {
        deleted: false,
        fields: BTreeMap::new(),
    };
    let mut conflicts = Vec::new();
    let mut notice = None;
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
                let seen: Vec<ChangeHash> =
                    seen.split(',').filter_map(|h| h.parse().ok()).collect();
                let resurrected = doc.get_changes(&seen).iter().any(|c| {
                    c.message()
                        .and_then(|m| m.split_once('\t'))
                        .is_some_and(|(o, f)| o == obj && f != DELETED)
                });
                view.deleted = !resurrected;
                if resurrected {
                    notice = Some(Notice {
                        obj: obj.to_owned(),
                        deleter: win_hlc.device,
                    });
                }
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
    Some((view, conflicts, notice))
}

impl AltA {
    fn doc(&mut self, circle: Circle) -> Result<&mut AutoCommit, Error> {
        self.docs
            .get_mut(&circle)
            .ok_or_else(|| format!("circle {circle} is not held by device {}", self.device))
    }

    fn view_of(&mut self, circle: Circle, obj: &str) -> Result<ObjView, Error> {
        let doc = self.doc(circle)?;
        derive(doc, obj)
            .map(|d| d.0)
            .ok_or_else(|| format!("no such object {obj}"))
    }

    fn touch(&mut self, circle: Circle, obj: &str) {
        let view = self
            .docs
            .get_mut(&circle)
            .and_then(|d| derive(d, obj))
            .map(|d| d.0);
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

    /// The document as bytes, as a device would persist it.
    pub fn save_circle(&mut self, circle: Circle) -> Result<Vec<u8>, Error> {
        Ok(self.doc(circle)?.save())
    }

    /// Replaces a circle's document by one loaded from `bytes` (a restart after a crash). Uses a
    /// fresh actor id: reusing the old one after losing local changes would reissue operation ids.
    pub fn load_circle(&mut self, circle: Circle, bytes: &[u8]) -> Result<(), Error> {
        let mut doc = AutoCommit::load(bytes).map_err(err)?;
        doc.set_actor(ActorId::random());
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
            index: Index::new(),
        }
    }

    fn device(&self) -> Device {
        self.device
    }

    fn apply_local(&mut self, circle: Circle, op: Op, now_ms: u64) -> Result<(), Error> {
        let touched = match op {
            Op::Create { obj, fields } => {
                if fields.is_empty() || fields.contains_key(DELETED) {
                    return Err("create needs fields, and not the delete register".into());
                }
                let doc = self.doc(circle)?;
                if doc.get(ROOT, obj.as_str()).map_err(err)?.is_some() {
                    return Err(format!("object {obj} exists"));
                }
                doc.put_object(ROOT, obj.as_str(), ObjType::Map)
                    .map_err(err)?;
                let writes: Vec<_> = fields.into_iter().collect();
                self.put_stamped(circle, &obj, &writes, now_ms, "*")?;
                obj
            }
            Op::Set { obj, field, value } => {
                if field == DELETED {
                    return Err("the delete register is not a field".into());
                }
                if self.view_of(circle, &obj)?.deleted {
                    return Err(format!("object {obj} is deleted"));
                }
                self.put_stamped(circle, &obj, &[(field.clone(), value)], now_ms, &field)?;
                obj
            }
            Op::Delete { obj } => {
                if self.view_of(circle, &obj)?.deleted {
                    return Err(format!("object {obj} is already deleted"));
                }
                let seen = self
                    .doc(circle)?
                    .get_heads()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                self.put_stamped(
                    circle,
                    &obj,
                    &[(DELETED.to_owned(), format!("1|{seen}"))],
                    now_ms,
                    DELETED,
                )?;
                obj
            }
            Op::Restore { obj } => {
                if !self.view_of(circle, &obj)?.deleted {
                    return Err(format!("object {obj} is not deleted"));
                }
                self.put_stamped(
                    circle,
                    &obj,
                    &[(DELETED.to_owned(), "0".to_owned())],
                    now_ms,
                    DELETED,
                )?;
                obj
            }
        };
        self.touch(circle, &touched);
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
        let doc = self.doc(circle)?;
        let heads = |d: &mut AutoCommit| -> Vec<String> {
            let mut h: Vec<String> = d.get_heads().iter().map(ToString::to_string).collect();
            h.sort();
            h
        };
        let before = heads(doc);
        let before_hashes = doc.get_heads();
        doc.sync()
            .receive_sync_message(&mut st.0, message)
            .map_err(err)?;
        let after = heads(doc);
        if before == after {
            return Ok(false);
        }
        let objs: BTreeSet<String> = doc
            .get_changes(&before_hashes)
            .iter()
            .filter_map(|c| Some(c.message()?.split_once('\t')?.0.to_owned()))
            .collect();
        for obj in objs {
            self.touch(circle, &obj);
        }
        Ok(true)
    }

    fn snapshot(&mut self, circle: Circle) -> Snapshot {
        let mut snap = Snapshot::default();
        let Some(doc) = self.docs.get_mut(&circle) else {
            return snap;
        };
        let objs: Vec<String> = doc.keys(ROOT).collect();
        for obj in objs {
            if let Some((view, conflicts, notice)) = derive(doc, &obj) {
                snap.conflicts.extend(conflicts);
                snap.notices.extend(notice);
                snap.objects.insert(obj, view);
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
            out.push_str(&format!("{:?}\n", self.snapshot(c)));
        }
        out.push_str(&format!("{:?}\n", self.index));
        out
    }
}
