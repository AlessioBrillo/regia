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
        let mut it = s.split('.');
        let mut part = |what: &str| {
            it.next()
                .ok_or_else(|| format!("hlc: missing {what}"))
                .map(str::to_owned)
        };
        let (ms, counter, device) = (part("ms")?, part("counter")?, part("device")?);
        let bad = |e: std::num::ParseIntError| format!("hlc: {e}");
        Ok(Self {
            ms: u64::from_str_radix(&ms, 16).map_err(bad)?,
            counter: u32::from_str_radix(&counter, 16).map_err(bad)?,
            device: u32::from_str_radix(&device, 16).map_err(bad)?,
        })
    }
}

/// The lww rule: among concurrent values (the heads of a register), the greatest [`Hlc`] wins and
/// the rest are returned as losers, greatest first. `None` for an empty register.
#[must_use]
pub fn pick<T: Clone>(heads: &[(Hlc, T)]) -> Option<((Hlc, T), Vec<(Hlc, T)>)> {
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
}

/// One visible object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjView {
    /// Effective deletion: the delete won and nothing it had not seen was written since.
    pub deleted: bool,
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

/// Everything a replica shows for one circle. Equal snapshots = converged.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    /// Objects by id.
    pub objects: BTreeMap<String, ObjView>,
    /// Unresolved conflicts.
    pub conflicts: BTreeSet<Conflict>,
    /// Pending notices.
    pub notices: BTreeSet<Notice>,
}

/// Local search index: `(circle, object) -> title` for every non-deleted object.
pub type Index = BTreeMap<(Circle, String), String>;

/// A client's local store and its sync endpoint. One implementation per DAT-01 alternative.
pub trait Replica: Sized {
    /// Per (peer, circle) sync session state; a fresh one per connection.
    type Sync: Default;

    /// A replica of `device` holding `circles`, and nothing else.
    fn new(device: Device, circles: &[Circle]) -> Self;
    /// Author of local writes.
    fn device(&self) -> Device;
    /// Apply a local op at simulated time `now_ms`.
    fn apply_local(&mut self, circle: Circle, op: Op, now_ms: u64) -> Result<(), Error>;
    /// Next sync message for this peer and circle, if any.
    fn generate(&mut self, circle: Circle, st: &mut Self::Sync) -> Option<Vec<u8>>;
    /// Process a peer's message. `true` if local state changed.
    fn receive(&mut self, circle: Circle, st: &mut Self::Sync, msg: &[u8]) -> Result<bool, Error>;
    /// Visible state of one circle.
    fn snapshot(&mut self, circle: Circle) -> Snapshot;
    /// The incrementally maintained local index.
    fn index(&self) -> Index;
    /// Drop the index and rebuild it from the source of truth (S-11).
    fn rebuild_index(&mut self);
    /// Bytes the store occupies (serialised document / database pages).
    fn storage_bytes(&mut self) -> usize;
    /// Every stored string, for the canary scan: stores, logs, conflicts, notices, indexes.
    fn dump_text(&mut self) -> String;
}

/// Index entry of one object from its visible state, if it should be indexed.
pub(crate) fn index_entry(view: &ObjView) -> Option<String> {
    if view.deleted {
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
