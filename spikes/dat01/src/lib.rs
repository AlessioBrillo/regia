//! DAT-01 spike, increment 1: two storage/sync alternatives behind one trait, driven by one
//! deterministic simulator, over the synthetic dataset of `regia_domain::synthetic`.
//!
//! Throwaway code: nothing here is a decision (ADR-002/003 are `proposed`). The conflict rules
//! below are the owner's rules *for the spike* (2026-11-12), not ratified semantics:
//!
//! - **Same field, concurrent edits (S-03):** the value with the greatest [`Hlc`] wins; every
//!   other concurrent value stays visible as a [`Conflict`] until someone writes the field again.
//! - **Delete vs concurrent edit (S-05):** an edit the delete had not seen resurrects the object,
//!   and the deleter gets a [`Notice`].
//!
//! Both alternatives derive their visible state from the same two rules, so any disagreement
//! between them is a bug in one adapter, which is what the differential test looks for.

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod alt_a;
pub mod alt_b;
pub mod alt_c;
pub mod backlog;
pub mod data;
pub mod sim;

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

/// Index of a circle in the dataset.
pub type Circle = usize;
/// A client device. In the simulator, device `d` acts for the dataset principal `d % P`.
pub type Device = u32;
/// Every fallible operation reports a plain message: this is a spike, not an API.
pub type Error = String;

/// Reserved field holding the delete/restore register of an object. `"1"` = deleted.
pub const DELETED: &str = "__deleted";
/// Reserved field of an object moved to another circle (S-10). Once written the object is gone
/// from the circle that holds it.
pub const MOVED: &str = "__moved";
/// Reserved object of a circle that holds its membership registers (S-09). Never shown.
pub const CIRCLE_OBJ: &str = "__circle";

/// Field of [`CIRCLE_OBJ`] that revokes `member`. Its value is the cut: how many of the member's
/// own writes the revoker had seen. The cut is a count of the author's own log, so it is causal
/// and no clock can move it.
#[must_use]
pub fn revoked_field(member: Device) -> String {
    format!("revoked:{member}")
}

/// Inverse of [`revoked_field`].
#[must_use]
pub fn parse_revoked(field: &str) -> Option<Device> {
    field.strip_prefix("revoked:")?.parse().ok()
}

/// Hybrid logical clock. Ordering is the conflict winner order: time, counter, then device id
/// (the deterministic tie-break).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Hlc {
    /// Milliseconds (simulated).
    pub ms: u64,
    /// Events within the same millisecond.
    pub counter: u32,
    /// Authoring device.
    pub device: Device,
}

impl Hlc {
    /// Next stamp for a local event at wall time `now_ms`, never going backwards.
    #[must_use]
    pub fn tick(self, now_ms: u64, device: Device) -> Self {
        if now_ms > self.ms {
            Self {
                ms: now_ms,
                counter: 0,
                device,
            }
        } else {
            Self {
                ms: self.ms,
                counter: self.counter + 1,
                device,
            }
        }
    }
}

/// A received stamp more than this far ahead of the receiver's clock is refused as skew.
pub const MAX_SKEW_MS: u64 = 60_000;

impl Hlc {
    /// Clock after seeing a remote stamp: never behind it, so the next local [`Hlc::tick`] is
    /// strictly after everything this replica has seen.
    #[must_use]
    pub fn recv(self, remote: Self) -> Self {
        self.max(remote)
    }

    /// Whether the stamp is implausibly far ahead of `now_ms`.
    #[must_use]
    pub const fn skewed(self, now_ms: u64) -> bool {
        self.ms > now_ms.saturating_add(MAX_SKEW_MS)
    }
}

impl fmt::Display for Hlc {
    /// Fixed-width hex: lexicographic order of the text equals [`Ord`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:016x}.{:08x}.{:08x}",
            self.ms, self.counter, self.device
        )
    }
}

impl std::str::FromStr for Hlc {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let parts: Vec<&str> = s.split('.').collect();
        let [ms, counter, device] = parts[..] else {
            return Err(format!("hlc: expected 3 parts in {s:?}"));
        };
        // Canonical width only: text order must equal `Ord`.
        if (ms.len(), counter.len(), device.len()) != (16, 8, 8) {
            return Err(format!("hlc: non-canonical width in {s:?}"));
        }
        let bad = |e: std::num::ParseIntError| format!("hlc: {e}");
        Ok(Self {
            ms: u64::from_str_radix(ms, 16).map_err(bad)?,
            counter: u32::from_str_radix(counter, 16).map_err(bad)?,
            device: u32::from_str_radix(device, 16).map_err(bad)?,
        })
    }
}

/// A value and the stamp it was written with.
pub type Stamped<T> = (Hlc, T);

/// The lww rule: among concurrent values (the heads of a register), the greatest [`Hlc`] wins and
/// the rest are returned as losers, greatest first. `None` for an empty register.
#[must_use]
pub fn pick<T: Clone>(heads: &[Stamped<T>]) -> Option<(Stamped<T>, Vec<Stamped<T>>)> {
    let mut sorted = heads.to_vec();
    sorted.sort_by_key(|(h, _)| std::cmp::Reverse(*h));
    let mut it = sorted.into_iter();
    it.next().map(|w| (w, it.collect()))
}

/// What a client can do to an object of a circle it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// New object with at least one field (`title`).
    Create {
        /// Stable object id (the dataset's UUID text).
        obj: String,
        /// Initial fields.
        fields: BTreeMap<String, String>,
    },
    /// Write one field of an existing, non-deleted object.
    Set {
        /// Object id.
        obj: String,
        /// Field name (not [`DELETED`]).
        field: String,
        /// New value.
        value: String,
    },
    /// Delete an object (it stays restorable).
    Delete {
        /// Object id.
        obj: String,
    },
    /// Undo a delete.
    Restore {
        /// Object id.
        obj: String,
    },
    /// Remove `member` from the circle (S-09). Everything of theirs the revoker has not seen is
    /// held aside. The relay stops syncing the circle with them (see `Sim::revoke`).
    Revoke {
        /// The device being revoked.
        member: Device,
    },
    /// Move an object from the circle it is applied in to `to` (S-10). The mover must hold both.
    Move {
        /// Object id.
        obj: String,
        /// Destination circle.
        to: Circle,
    },
}

/// One visible object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjView {
    /// Effective deletion: the delete won and nothing it had not seen was written since.
    pub deleted: bool,
    /// The object was moved to another circle. A snapshot omits such objects.
    pub moved: bool,
    /// Winning value per field (the delete register is not a field).
    pub fields: BTreeMap<String, String>,
}

/// A concurrent value that lost, kept so nobody loses intent silently.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Conflict {
    /// Object id.
    pub obj: String,
    /// Field name.
    pub field: String,
    /// The value that lost.
    pub lost: String,
    /// Its stamp (so identical values from different devices stay distinct).
    pub hlc: String,
}

/// "Your delete was overridden by an edit you had not seen."
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Notice {
    /// Object id.
    pub obj: String,
    /// Device whose delete was overridden.
    pub deleter: Device,
}

/// Why a write is held aside instead of shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// Its author was revoked and the revoker had not seen it (S-09).
    Revoked,
    /// It was made in the old circle by someone who had not seen the object move (S-10).
    MovedAway,
    /// It was built on a revoked author's unseen change, which cannot be removed from the history
    /// without it (Automerge only).
    Dependent,
}

/// A write held aside: not visible, not merged into any object, not lost. The author sees the
/// same entry once it syncs, which is how they are told.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quarantined {
    /// Object id.
    pub obj: String,
    /// Device that wrote it.
    pub author: Device,
    /// Why.
    pub reason: Reason,
}

/// Field names added by schema v1 (S-06). A v0 replica does not know them.
pub const V1_FIELDS: &[&str] = &["area"];

/// Schema a replica understands (S-06). Both versions store and forward every field; v0 only
/// hides and refuses the fields of [`V1_FIELDS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Schema {
    /// Knows no field of [`V1_FIELDS`].
    #[default]
    V0,
    /// Knows every field.
    V1,
}

impl Schema {
    /// Whether a local write to `field` is allowed.
    pub fn check_write(self, field: &str) -> Result<(), Error> {
        if self == Self::V0 && V1_FIELDS.contains(&field) {
            return Err(format!("field {field:?} needs a newer app: update the app"));
        }
        Ok(())
    }

    /// Show only what this schema understands. An object that had other fields is listed in
    /// [`Snapshot::needs_update`]; the fields themselves stay in the store.
    pub(crate) fn mask(self, snap: &mut Snapshot) {
        if self == Self::V1 {
            return;
        }
        for (obj, view) in &mut snap.objects {
            let before = view.fields.len();
            view.fields.retain(|f, _| !V1_FIELDS.contains(&f.as_str()));
            if view.fields.len() != before {
                snap.needs_update.insert(obj.clone());
            }
        }
        for c in &snap.conflicts {
            if V1_FIELDS.contains(&c.field.as_str()) {
                snap.needs_update.insert(c.obj.clone());
            }
        }
        snap.conflicts
            .retain(|c| !V1_FIELDS.contains(&c.field.as_str()));
    }
}

/// Everything a replica shows for one circle. Equal snapshots = converged.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    /// Objects by id.
    pub objects: BTreeMap<String, ObjView>,
    /// Unresolved conflicts.
    pub conflicts: BTreeSet<Conflict>,
    /// Pending notices.
    pub notices: BTreeSet<Notice>,
    /// Writes held aside for an admin (S-09, S-10).
    pub quarantine: BTreeSet<Quarantined>,
    /// Objects holding fields this replica's [`Schema`] does not know: "update the app" (S-06).
    pub needs_update: BTreeSet<String>,
}

/// Local search index: `(circle, object) -> title` for every non-deleted object.
pub type Index = BTreeMap<(Circle, String), String>;

/// A client's local store and its sync endpoint. One implementation per DAT-01 alternative.
pub trait Replica: Sized {
    /// Per (peer, circle) sync session state; a fresh one per connection.
    type Sync: Default;

    /// A replica of `device` holding `circles`, and nothing else.
    fn new(device: Device, circles: &[Circle]) -> Self;
    /// Choose the schema this replica understands (default [`Schema::V0`]).
    fn set_schema(&mut self, schema: Schema);
    /// A replica of `device` that understands `schema`.
    #[must_use]
    fn new_versioned(device: Device, circles: &[Circle], schema: Schema) -> Self {
        let mut r = Self::new(device, circles);
        r.set_schema(schema);
        r
    }
    /// Author of local writes.
    fn device(&self) -> Device;
    /// Apply a local op at simulated time `now_ms`.
    fn apply_local(&mut self, circle: Circle, op: Op, now_ms: u64) -> Result<(), Error>;
    /// Receiver's idea of the true time, set by the transport before a session. Until it is set
    /// no skew check is made.
    fn set_now(&mut self, now_ms: u64);
    /// Next sync message for this peer and circle, if any.
    fn generate(&mut self, circle: Circle, st: &mut Self::Sync) -> Option<Vec<u8>>;
    /// Process a peer's message. `true` if local state changed.
    fn receive(&mut self, circle: Circle, st: &mut Self::Sync, msg: &[u8]) -> Result<bool, Error>;
    /// Visible state of one circle.
    fn snapshot(&mut self, circle: Circle) -> Snapshot;
    /// The incrementally maintained local index.
    fn index(&self) -> Index;
    /// This device's whole store of `circle` as one message: what a revoked device sends when it
    /// returns (S-09). It receives nothing back.
    fn upload(&mut self, circle: Circle) -> Vec<u8>;
    /// Merge an [`upload`](Replica::upload) of a revoked device. Same checks as a sync message,
    /// including the skew refusal. `true` if local state changed.
    fn accept_upload(&mut self, circle: Circle, msg: &[u8]) -> Result<bool, Error>;
    /// Drop the index and rebuild it from the source of truth (S-11).
    fn rebuild_index(&mut self);
    /// Bytes the store occupies (serialised document / database pages).
    fn storage_bytes(&mut self) -> usize;
    /// Every stored string, for the canary scan: stores, logs, conflicts, notices, indexes.
    fn dump_text(&mut self) -> String;
}

/// Index entry of one object from its visible state, if it should be indexed.
pub(crate) fn index_entry(view: &ObjView) -> Option<String> {
    if view.deleted || view.moved {
        None
    } else {
        view.fields.get("title").cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hlc_text_roundtrip_and_order() {
        let a = Hlc {
            ms: 5,
            counter: 0,
            device: 1,
        };
        let b = Hlc {
            ms: 5,
            counter: 0,
            device: 2,
        };
        let c = Hlc {
            ms: 255,
            counter: 3,
            device: 0,
        };
        for h in [a, b, c] {
            assert_eq!(h, h.to_string().parse().unwrap());
        }
        assert!(a < b && b < c);
        assert!(a.to_string() < b.to_string() && b.to_string() < c.to_string());
    }

    #[test]
    fn recv_then_tick_is_after_the_remote_stamp() {
        let local = Hlc {
            ms: 5,
            counter: 0,
            device: 1,
        };
        let remote = Hlc {
            ms: 900,
            counter: 2,
            device: 7,
        };
        assert!(local.recv(remote).tick(6, 1) > remote);
        let far = Hlc {
            ms: 100_000,
            ..remote
        };
        assert!(!far.skewed(100_000 - MAX_SKEW_MS) && far.skewed(100_000 - MAX_SKEW_MS - 1));
    }

    #[test]
    fn tick_never_goes_backwards() {
        let h = Hlc {
            ms: 10,
            counter: 0,
            device: 1,
        };
        assert!(h.tick(5, 1) > h);
        assert!(h.tick(11, 1) > h);
        assert_eq!(h.tick(10, 1).counter, 1);
    }

    #[test]
    fn pick_greatest_hlc_wins_ties_by_device() {
        let h = |ms, device| Hlc {
            ms,
            counter: 0,
            device,
        };
        let ((w, v), losers) = pick(&[(h(1, 9), "old"), (h(2, 1), "a"), (h(2, 3), "b")]).unwrap();
        assert_eq!((w, v), (h(2, 3), "b"));
        assert_eq!(
            losers.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
            ["a", "old"]
        );
        assert!(pick::<&str>(&[]).is_none());
    }
}
