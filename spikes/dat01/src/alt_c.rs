//! Alternative C: hybrid. Documents and notes live in Automerge ([`AltA`]); everything else in the
//! SQLite op log ([`AltB`]). Split frozen by the owner in the experiment card (2026-11-14).
//!
//! C owns no storage and no merge logic: it routes each op to one side and frames the two sides'
//! sync messages together. Routing is by the `kind` field given at creation (`document` or
//! `note` go to A, anything else, or no kind, to B); an object never changes side.
//!
//! Known limits (recorded in the results, not hidden): the two sides commit separately, so a
//! failure between them can leave one applied (a `Revoke` is applied on both); a message whose
//! A half is accepted and B half refused is applied half.

use std::collections::BTreeMap;

use crate::{
    Circle, Device, Error, Index, Op, Replica, Schema, Snapshot,
    alt_a::{ASync, AltA},
    alt_b::{AltB, BSync},
};

/// Per (peer, circle) session state: one per side.
#[derive(Default)]
pub struct CSync {
    a: ASync,
    b: BSync,
}

/// Replica holding both stores.
pub struct AltC {
    a: AltA,
    b: AltB,
    /// Side of every object seen so far: `true` = A. Filled on create and by probing.
    side: BTreeMap<String, bool>,
}

/// Kinds stored in Automerge.
fn on_a(kind: Option<&String>) -> bool {
    matches!(kind.map(String::as_str), Some("document" | "note"))
}

/// One frame: flags (bit 0 = A present, bit 1 = B present), then A as `u32` length + bytes, then
/// B as the rest. Binary on purpose: JSON would inflate the bytes S-08 counts.
fn pack(a: Option<Vec<u8>>, b: Option<Vec<u8>>) -> Vec<u8> {
    let mut out = vec![u8::from(a.is_some()) | (u8::from(b.is_some()) << 1)];
    if let Some(a) = a {
        out.extend_from_slice(&u32::try_from(a.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(&a);
    }
    if let Some(b) = b {
        out.extend_from_slice(&b);
    }
    out
}

type Halves<'a> = (Option<&'a [u8]>, Option<&'a [u8]>);

fn unpack(msg: &[u8]) -> Result<Halves<'_>, Error> {
    let bad = || Error::from("hybrid frame: malformed");
    let (&flags, mut rest) = msg.split_first().ok_or_else(bad)?;
    let a = if flags & 1 == 1 {
        let (len, tail) = rest.split_first_chunk::<4>().ok_or_else(bad)?;
        let n = u32::from_le_bytes(*len) as usize;
        let (a, tail) = tail.split_at_checked(n).ok_or_else(bad)?;
        rest = tail;
        Some(a)
    } else {
        None
    };
    Ok((a, (flags & 2 == 2).then_some(rest)))
}

impl AltC {
    /// Side holding `obj`: the cache, else whichever store shows it. An object neither store
    /// knows yet is not cached (the differential test found that caching the guess misroutes
    /// the object once a peer's create arrives); the op then fails on B as "no such object".
    fn owner(&mut self, circle: Circle, obj: &str) -> bool {
        if let Some(&s) = self.side.get(obj) {
            return s;
        }
        let found = if self.a.snapshot(circle).objects.contains_key(obj) {
            Some(true)
        } else {
            self.b
                .snapshot(circle)
                .objects
                .contains_key(obj)
                .then_some(false)
        };
        if let Some(s) = found {
            self.side.insert(obj.to_owned(), s);
        }
        found.unwrap_or(false)
    }
}

impl Replica for AltC {
    type Sync = CSync;

    fn new(device: Device, circles: &[Circle]) -> Self {
        Self {
            a: AltA::new(device, circles),
            b: AltB::new(device, circles),
            side: BTreeMap::new(),
        }
    }

    fn set_schema(&mut self, schema: Schema) {
        self.a.set_schema(schema);
        self.b.set_schema(schema);
    }

    fn device(&self) -> Device {
        self.a.device()
    }

    fn set_now(&mut self, now_ms: u64) {
        self.a.set_now(now_ms);
        self.b.set_now(now_ms);
    }

    fn apply_local(&mut self, circle: Circle, op: Op, now_ms: u64) -> Result<(), Error> {
        let (side, created) = match &op {
            Op::Create { obj, fields } => (on_a(fields.get("kind")), Some(obj.clone())),
            Op::Set { obj, .. }
            | Op::Delete { obj }
            | Op::Restore { obj }
            | Op::Move { obj, .. } => (self.owner(circle, obj), None),
            Op::Revoke { .. } => {
                // Membership registers live in both stores.
                self.b.apply_local(circle, op.clone(), now_ms)?;
                return self.a.apply_local(circle, op, now_ms);
            }
        };
        if side {
            self.a.apply_local(circle, op, now_ms)?;
        } else {
            self.b.apply_local(circle, op, now_ms)?;
        }
        if let Some(obj) = created {
            self.side.insert(obj, side); // only once the create succeeded
        }
        Ok(())
    }

    fn generate(&mut self, circle: Circle, st: &mut CSync) -> Option<Vec<u8>> {
        let a = self.a.generate(circle, &mut st.a);
        let b = self.b.generate(circle, &mut st.b);
        (a.is_some() || b.is_some()).then(|| pack(a, b))
    }

    fn receive(&mut self, circle: Circle, st: &mut CSync, msg: &[u8]) -> Result<bool, Error> {
        let (a, b) = unpack(msg)?;
        let mut changed = false;
        if let Some(a) = a {
            changed |= self.a.receive(circle, &mut st.a, a)?;
        }
        if let Some(b) = b {
            changed |= self.b.receive(circle, &mut st.b, b)?;
        }
        Ok(changed)
    }

    fn snapshot(&mut self, circle: Circle) -> Snapshot {
        let mut s = self.a.snapshot(circle);
        let b = self.b.snapshot(circle);
        s.objects.extend(b.objects);
        s.conflicts.extend(b.conflicts);
        s.notices.extend(b.notices);
        s.quarantine.extend(b.quarantine);
        s.needs_update.extend(b.needs_update);
        s
    }

    fn index(&self) -> Index {
        let mut i = self.a.index();
        i.extend(self.b.index());
        i
    }

    fn upload(&mut self, circle: Circle) -> Vec<u8> {
        pack(Some(self.a.upload(circle)), Some(self.b.upload(circle)))
    }

    fn accept_upload(&mut self, circle: Circle, msg: &[u8]) -> Result<bool, Error> {
        let (Some(a), Some(b)) = unpack(msg)? else {
            return Err("hybrid upload needs both halves".into());
        };
        Ok(self.a.accept_upload(circle, a)? | self.b.accept_upload(circle, b)?)
    }

    fn rebuild_index(&mut self) {
        self.a.rebuild_index();
        self.b.rebuild_index();
    }

    fn storage_bytes(&mut self) -> usize {
        self.a.storage_bytes() + self.b.storage_bytes()
    }

    fn dump_text(&mut self) -> String {
        format!("{}{}", self.a.dump_text(), self.b.dump_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip_all_shapes() {
        for (a, b) in [
            (None, None),
            (Some(vec![1, 2, 3]), None),
            (None, Some(vec![9, 9])),
            (Some(vec![]), Some(vec![7])),
            (Some(vec![1; 300]), Some(vec![2; 5])),
        ] {
            let frame = pack(a.clone(), b.clone());
            let (ra, rb) = unpack(&frame).unwrap();
            assert_eq!((ra.map(<[u8]>::to_vec), rb.map(<[u8]>::to_vec)), (a, b));
        }
        assert!(unpack(&[]).is_err());
        assert!(unpack(&[1, 9, 0, 0, 0, 1]).is_err(), "length past the end");
    }
}
