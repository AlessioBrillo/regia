//! DAT-01 increment 1: S-01..S-05, S-07, S-11, the canary check, and a differential property
//! test (A and B must show the same visible state for the same history). Synthetic data only.
//!
//! Every scenario runs against both alternatives through the [`Replica`] trait.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use regia_domain::synthetic::SMALL;
use regia_spike_dat01::{
    Circle, Conflict, Device, Notice, Op, Replica,
    alt_a::AltA,
    alt_b::AltB,
    data::{Dataset, Item},
    sim::Sim,
};

const DEVICES: u32 = 3;

struct World<R: Replica> {
    ds: Dataset,
    sim: Sim<R>,
}

fn world<R: Replica>() -> World<R> {
    let ds = Dataset::load(42, &SMALL);
    let scope = (0..DEVICES).map(|d| ds.circles_of(d)).collect();
    World {
        sim: Sim::new(scope),
        ds,
    }
}

fn fields(item: &Item) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("title".to_owned(), item.title.clone()),
        ("status".to_owned(), "open".to_owned()),
    ])
}

impl<R: Replica> World<R> {
    fn create(&mut self, d: Device, item: &Item) {
        let op = Op::Create {
            obj: item.id.clone(),
            fields: fields(item),
        };
        self.sim.op(d, item.circle, op).unwrap();
    }

    fn set(
        &mut self,
        d: Device,
        circle: Circle,
        obj: &str,
        field: &str,
        value: &str,
    ) -> Result<(), String> {
        let op = Op::Set {
            obj: obj.to_owned(),
            field: field.to_owned(),
            value: value.to_owned(),
        };
        self.sim.op(d, circle, op)
    }

    /// A circle with at least two member devices, those two, and its first item.
    fn pair(&self) -> (Item, Device, Device) {
        for item in &self.ds.items {
            let m = self.sim.members(item.circle);
            if m.len() >= 2 {
                return (item.clone(), m[0], m[1]);
            }
        }
        panic!("no circle with two member devices");
    }

    /// Every item created by one of its circle's member devices, round-robin. Nothing synced yet.
    fn populate(&mut self) -> Vec<(Item, Device)> {
        let mut made = Vec::new();
        for (i, item) in self.ds.items.clone().iter().enumerate() {
            let m = self.sim.members(item.circle);
            if m.is_empty() {
                continue;
            }
            let d = m[i % m.len()];
            self.create(d, item);
            made.push((item.clone(), d));
        }
        made
    }

    fn assert_converged(&mut self) {
        for c in 0..self.ds.circles {
            let members = self.sim.members(c);
            let snaps: Vec<_> = members
                .iter()
                .map(|&d| self.sim.clients[d as usize].snapshot(c))
                .collect();
            assert!(
                snaps.windows(2).all(|w| w[0] == w[1]),
                "circle {c} diverged"
            );
        }
    }

    /// AR-04: no canary of a circle reaches a device outside it, in any store, log, conflict,
    /// notice or index. Positive control: a member does hold its own canaries.
    fn assert_no_leak(&mut self, made: &[(Item, Device)]) {
        let (mut checked, mut seen_by_member) = (0, 0);
        for (item, _) in made.iter().filter(|(i, _)| Dataset::is_canary(i)) {
            let tag = &item.title[item.title.find("CANARY-").unwrap()..];
            for d in 0..DEVICES {
                let text = self.sim.clients[d as usize].dump_text();
                if self.sim.scope_of(d).contains(&item.circle) {
                    seen_by_member += usize::from(text.contains(tag));
                } else {
                    checked += 1;
                    assert!(!text.contains(tag), "canary {tag} leaked to device {d}");
                }
            }
        }
        assert!(
            checked > 0,
            "no canary was ever outside a device's circles: the check proves nothing"
        );
        assert!(
            seen_by_member > 0,
            "members never held a canary: the scan proves nothing"
        );
    }
}

// --- S-01 ------------------------------------------------------------------------------------

fn s01_offline_writes_then_reconnect<R: Replica>() {
    let mut w = world::<R>();
    let made = w.populate();
    for (i, (item, d)) in made.iter().enumerate() {
        if i % 3 == 0 {
            w.set(*d, item.circle, &item.id, "status", "done").unwrap();
        }
    }
    w.sim.sync_all().unwrap();
    w.assert_converged();
    for (i, (item, _)) in made.iter().enumerate() {
        for d in w.sim.members(item.circle) {
            let snap = w.sim.clients[d as usize].snapshot(item.circle);
            let view = &snap.objects[&item.id];
            assert_eq!(view.fields["title"], item.title);
            assert_eq!(
                view.fields["status"],
                if i % 3 == 0 { "done" } else { "open" }
            );
            assert!(snap.conflicts.is_empty() && snap.notices.is_empty());
        }
    }
    w.assert_no_leak(&made);
}

// --- S-02, S-03 --------------------------------------------------------------------------------

fn s02_different_fields_both_kept<R: Replica>() {
    let mut w = world::<R>();
    let (item, a, b) = w.pair();
    w.create(a, &item);
    w.sim.sync_all().unwrap();
    w.set(a, item.circle, &item.id, "title", "A-edit").unwrap();
    w.set(b, item.circle, &item.id, "status", "B-edit").unwrap();
    w.sim.sync_all().unwrap();
    for d in [a, b] {
        let snap = w.sim.clients[d as usize].snapshot(item.circle);
        assert_eq!(snap.objects[&item.id].fields["title"], "A-edit");
        assert_eq!(snap.objects[&item.id].fields["status"], "B-edit");
        assert!(snap.conflicts.is_empty());
    }
}

fn s03_same_field_lww_with_visible_conflict<R: Replica>() {
    let mut w = world::<R>();
    let (item, a, b) = w.pair();
    w.create(a, &item);
    w.sim.sync_all().unwrap();
    // Concurrent: neither has seen the other's edit. b writes later, so b wins.
    w.set(a, item.circle, &item.id, "title", "X").unwrap();
    w.set(b, item.circle, &item.id, "title", "Y").unwrap();
    w.sim.sync_all().unwrap();
    w.assert_converged();
    let snap = w.sim.clients[a as usize].snapshot(item.circle);
    assert_eq!(snap.objects[&item.id].fields["title"], "Y");
    let lost: Vec<&Conflict> = snap.conflicts.iter().filter(|c| c.obj == item.id).collect();
    assert_eq!(lost.len(), 1, "exactly one visible conflict");
    assert_eq!(
        (lost[0].field.as_str(), lost[0].lost.as_str()),
        ("title", "X")
    );
    // Restorable: writing the lost value again resolves the conflict.
    w.set(a, item.circle, &item.id, "title", "X").unwrap();
    w.sim.sync_all().unwrap();
    for d in [a, b] {
        let snap = w.sim.clients[d as usize].snapshot(item.circle);
        assert_eq!(snap.objects[&item.id].fields["title"], "X");
        assert!(snap.conflicts.is_empty());
    }
}

fn s03_equal_stamps_break_ties_by_device<R: Replica>() {
    let mut w = world::<R>();
    let (item, a, b) = w.pair();
    assert!(a < b);
    w.create(a, &item);
    w.sim.sync_all().unwrap();
    let t = w.sim.now + 10_000;
    w.sim
        .op_at(
            a,
            item.circle,
            Op::Set {
                obj: item.id.clone(),
                field: "title".into(),
                value: "from-a".into(),
            },
            t,
        )
        .unwrap();
    w.sim
        .op_at(
            b,
            item.circle,
            Op::Set {
                obj: item.id.clone(),
                field: "title".into(),
                value: "from-b".into(),
            },
            t,
        )
        .unwrap();
    w.sim.sync_all().unwrap();
    for d in [a, b] {
        let snap = w.sim.clients[d as usize].snapshot(item.circle);
        assert_eq!(snap.objects[&item.id].fields["title"], "from-b");
        assert_eq!(snap.conflicts.len(), 1);
    }
}

// --- S-04, S-05 --------------------------------------------------------------------------------

fn s04_delete_then_restore_equals_before<R: Replica>() {
    let mut w = world::<R>();
    let (item, a, b) = w.pair();
    w.create(a, &item);
    w.sim.sync_all().unwrap();
    let before = w.sim.clients[b as usize].snapshot(item.circle);
    let key = (item.circle, item.id.clone());
    assert!(w.sim.clients[a as usize].index().contains_key(&key));

    w.sim
        .op(
            a,
            item.circle,
            Op::Delete {
                obj: item.id.clone(),
            },
        )
        .unwrap();
    w.sim.sync_all().unwrap();
    for d in [a, b] {
        assert!(w.sim.clients[d as usize].snapshot(item.circle).objects[&item.id].deleted);
        assert!(!w.sim.clients[d as usize].index().contains_key(&key));
    }
    // A deleted object cannot be edited.
    assert!(w.set(b, item.circle, &item.id, "title", "nope").is_err());

    w.sim
        .op(
            b,
            item.circle,
            Op::Restore {
                obj: item.id.clone(),
            },
        )
        .unwrap();
    w.sim.sync_all().unwrap();
    for d in [a, b] {
        assert_eq!(w.sim.clients[d as usize].snapshot(item.circle), before);
        assert!(w.sim.clients[d as usize].index().contains_key(&key));
    }
}

fn s05_edit_unseen_by_delete_resurrects_and_notifies<R: Replica>() {
    let mut w = world::<R>();
    let (item, a, b) = w.pair();
    w.create(a, &item);
    w.sim.sync_all().unwrap();
    w.sim
        .op(
            a,
            item.circle,
            Op::Delete {
                obj: item.id.clone(),
            },
        )
        .unwrap();
    w.set(b, item.circle, &item.id, "title", "B-edit").unwrap(); // b has not seen the delete
    w.sim.sync_all().unwrap();
    w.assert_converged();
    let key = (item.circle, item.id.clone());
    for d in [a, b] {
        let snap = w.sim.clients[d as usize].snapshot(item.circle);
        let view = &snap.objects[&item.id];
        assert!(!view.deleted, "the edit resurrects the object");
        assert_eq!(view.fields["title"], "B-edit");
        assert_eq!(
            snap.notices,
            BTreeSet::from([Notice {
                obj: item.id.clone(),
                deleter: a
            }])
        );
        assert_eq!(w.sim.clients[d as usize].index()[&key], "B-edit");
    }
    // The deleter, now seeing everything, can delete again, and this time it sticks.
    w.sim
        .op(
            a,
            item.circle,
            Op::Delete {
                obj: item.id.clone(),
            },
        )
        .unwrap();
    w.sim.sync_all().unwrap();
    for d in [a, b] {
        let snap = w.sim.clients[d as usize].snapshot(item.circle);
        assert!(snap.objects[&item.id].deleted && snap.notices.is_empty());
    }
}

// --- S-11 --------------------------------------------------------------------------------------

fn s11_index_rebuild_is_identical<R: Replica>() {
    let mut w = world::<R>();
    let made = w.populate();
    w.sim.sync_all().unwrap();
    // Deletes, a restore and a resurrection, so the index has something to get wrong.
    for (i, (item, d)) in made.iter().enumerate().filter(|(i, _)| i % 4 == 0) {
        let peers = w.sim.members(item.circle);
        w.sim
            .op(
                *d,
                item.circle,
                Op::Delete {
                    obj: item.id.clone(),
                },
            )
            .unwrap();
        match (i / 4) % 3 {
            0 => {
                w.sim.sync_all().unwrap();
                w.sim
                    .op(
                        *d,
                        item.circle,
                        Op::Restore {
                            obj: item.id.clone(),
                        },
                    )
                    .unwrap();
            }
            1 => {
                if let Some(&other) = peers.iter().find(|&&p| p != *d) {
                    w.set(other, item.circle, &item.id, "title", "late edit")
                        .unwrap();
                }
            }
            _ => {}
        }
    }
    w.sim.sync_all().unwrap();
    for d in 0..DEVICES {
        let client = &mut w.sim.clients[d as usize];
        let incremental = client.index();
        client.rebuild_index();
        assert_eq!(
            incremental,
            client.index(),
            "device {d}: incremental index differs from rebuild"
        );
        for c in client.index().keys().map(|k| k.0).collect::<BTreeSet<_>>() {
            let snap = client.snapshot(c);
            for (obj, view) in &snap.objects {
                assert_eq!(
                    !view.deleted,
                    client.index().contains_key(&(c, obj.clone()))
                );
            }
        }
    }
    w.assert_no_leak(&made);
}

// --- S-07 ----------------------------------------------------------------------------------------

/// Runs a full sync session between two replicas outside a `Sim` (they are built by hand).
fn session<R: Replica>(a: &mut R, b: &mut R, circle: Circle) {
    let (mut sa, mut sb) = (R::Sync::default(), R::Sync::default());
    for _ in 0..64 {
        let (ma, mb) = (a.generate(circle, &mut sa), b.generate(circle, &mut sb));
        if ma.is_none() && mb.is_none() {
            return;
        }
        if let Some(m) = ma {
            b.receive(circle, &mut sb, &m).unwrap();
        }
        if let Some(m) = mb {
            a.receive(circle, &mut sa, &m).unwrap();
        }
    }
    panic!("session did not settle");
}

fn seeded_writer<R: Replica>(circle: Circle) -> R {
    let mut y = R::new(1, &[circle]);
    for n in 0..4 {
        let f = BTreeMap::from([("title".to_owned(), format!("obj {n}"))]);
        y.apply_local(
            circle,
            Op::Create {
                obj: format!("o{n}"),
                fields: f,
            },
            10 + n,
        )
        .unwrap();
    }
    y.apply_local(
        circle,
        Op::Set {
            obj: "o0".into(),
            field: "title".into(),
            value: "edited".into(),
        },
        50,
    )
    .unwrap();
    y.apply_local(circle, Op::Delete { obj: "o1".into() }, 60)
        .unwrap();
    y
}

#[test]
fn s07_b_aborted_import_leaves_no_torn_state_and_resyncs() {
    let path = std::env::temp_dir().join(format!("regia-dat01-s07-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut y: AltB = seeded_writer(0);
    let mut x = AltB::open(0, &[0], &path).unwrap();
    let ops = y.ops_after(0, &x.vv(0));
    assert!(ops.len() > 4);
    for apply in 0..=ops.len() {
        x.abandon_import(0, &ops, apply).unwrap();
        drop(x);
        x = AltB::open(0, &[0], &path).unwrap(); // restart
        assert!(
            x.snapshot(0).objects.is_empty(),
            "torn import visible after {apply} ops"
        );
        assert!(x.index().is_empty());
    }
    session(&mut x, &mut y, 0);
    assert_eq!(x.snapshot(0), y.snapshot(0));
    drop(x);
    let mut reopened = AltB::open(0, &[0], &path).unwrap();
    assert_eq!(
        reopened.snapshot(0),
        y.snapshot(0),
        "committed state survives a restart"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn s07_a_truncated_save_is_refused_or_a_prefix_and_always_resyncs() {
    let mut y: AltA = seeded_writer(0);
    let mut x = AltA::new(0, &[0]);
    session(&mut x, &mut y, 0);
    let bytes = x.save_circle(0).unwrap();
    let full = y.snapshot(0);
    let mut reloaded = AltA::new(0, &[0]);
    reloaded.load_circle(0, &bytes).unwrap();
    assert_eq!(reloaded.snapshot(0), full);

    let step = (bytes.len() / 40).max(1);
    let (mut refused, mut partial) = (0, 0);
    for cut in (0..bytes.len()).step_by(step) {
        let mut r = AltA::new(0, &[0]);
        if r.load_circle(0, &bytes[..cut]).is_err() {
            refused += 1;
        } else {
            partial += 1;
            for (id, view) in &r.snapshot(0).objects {
                assert!(full.objects.contains_key(id), "invented object {id}");
                assert!(view.fields.contains_key("title"), "torn object {id}");
            }
        }
        session(&mut r, &mut y, 0);
        assert_eq!(r.snapshot(0), full, "cut at {cut} did not resync");
    }
    assert!(refused + partial > 0);
}

// --- Isolation, enforced by the replica itself -----------------------------------------------

/// The simulator never connects non-members, so `assert_no_leak` alone would pass by construction.
/// Here genuine messages of a circle are delivered directly to a replica that does not hold it.
fn foreign_circle_messages_are_refused<R: Replica>() {
    let mut y = R::new(1, &[0, 1]);
    let fields = BTreeMap::from([("title".to_owned(), "CANARY-iso-1".to_owned())]);
    y.apply_local(
        1,
        Op::Create {
            obj: "o".into(),
            fields,
        },
        10,
    )
    .unwrap();
    let mut z = R::new(2, &[1]);
    let (mut sy, mut sz) = (R::Sync::default(), R::Sync::default());
    let mut captured = Vec::new();
    for _ in 0..8 {
        let (my, mz) = (y.generate(1, &mut sy), z.generate(1, &mut sz));
        if my.is_none() && mz.is_none() {
            break;
        }
        if let Some(m) = my {
            captured.push(m.clone());
            z.receive(1, &mut sz, &m).unwrap();
        }
        if let Some(m) = mz {
            y.receive(1, &mut sy, &m).unwrap();
        }
    }
    assert!(captured.len() >= 2, "need the handshake and the data");
    let mut x = R::new(0, &[0]);
    let mut sx = R::Sync::default();
    for m in &captured {
        assert!(
            x.receive(1, &mut sx, m).is_err(),
            "a foreign-circle message was accepted"
        );
    }
    assert!(!x.dump_text().contains("CANARY-iso-1"));
}

// --- Differential property ---------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Act {
    Create(usize),
    Set(Device, usize, usize, u8),
    Delete(Device, usize),
    Restore(Device, usize),
    Sync(Device, Device),
    Offline(Device),
    Online(Device),
}

fn act() -> impl Strategy<Value = Act> {
    let (d, o) = (0..DEVICES, 0..4usize);
    prop_oneof![
        o.clone().prop_map(Act::Create),
        (d.clone(), o.clone(), 0..2usize, 0..6u8).prop_map(|(d, o, f, v)| Act::Set(d, o, f, v)),
        (d.clone(), o.clone()).prop_map(|(d, o)| Act::Delete(d, o)),
        (d.clone(), o).prop_map(|(d, o)| Act::Restore(d, o)),
        (d.clone(), d.clone()).prop_map(|(a, b)| Act::Sync(a, b)),
        d.clone().prop_map(Act::Offline),
        d.prop_map(Act::Online),
    ]
}

fn run<R: Replica>(acts: &[Act]) -> (Vec<bool>, Vec<regia_spike_dat01::Snapshot>) {
    let mut sim = Sim::<R>::new(vec![BTreeSet::from([0]); DEVICES as usize]);
    let mut outcomes = Vec::new();
    for a in acts {
        let r = match a {
            // Object k is created only by device k % 3: ids never collide, as UUIDs do not.
            Act::Create(k) => {
                let f = BTreeMap::from([
                    ("title".to_owned(), format!("t{k}")),
                    ("status".to_owned(), "open".to_owned()),
                ]);
                sim.op(
                    u32::try_from(k % 3).unwrap(),
                    0,
                    Op::Create {
                        obj: format!("o{k}"),
                        fields: f,
                    },
                )
            }
            Act::Set(d, k, f, v) => sim.op(
                *d,
                0,
                Op::Set {
                    obj: format!("o{k}"),
                    field: ["title", "status"][*f].to_owned(),
                    value: format!("v{v}"),
                },
            ),
            Act::Delete(d, k) => sim.op(
                *d,
                0,
                Op::Delete {
                    obj: format!("o{k}"),
                },
            ),
            Act::Restore(d, k) => sim.op(
                *d,
                0,
                Op::Restore {
                    obj: format!("o{k}"),
                },
            ),
            Act::Sync(a, b) => sim.sync_pair(*a, *b).map(|_| ()),
            Act::Offline(d) => {
                sim.set_online(*d, false);
                Ok(())
            }
            Act::Online(d) => {
                sim.set_online(*d, true);
                Ok(())
            }
        };
        outcomes.push(r.is_ok());
    }
    for d in 0..DEVICES {
        sim.set_online(d, true);
    }
    sim.sync_all().unwrap();
    let snaps = (0..DEVICES)
        .map(|d| sim.clients[d as usize].snapshot(0))
        .collect();
    for d in 0..DEVICES {
        let c = &mut sim.clients[d as usize];
        let inc = c.index();
        c.rebuild_index();
        assert_eq!(inc, c.index(), "incremental index != rebuild");
    }
    (outcomes, snaps)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]

    /// Same history on A and B: same accepted/refused ops, converged replicas, equal visible state.
    #[test]
    fn a_and_b_agree_and_converge(acts in prop::collection::vec(act(), 1..40)) {
        let (oa, sa) = run::<AltA>(&acts);
        let (ob, sb) = run::<AltB>(&acts);
        prop_assert_eq!(&oa, &ob, "accepted/refused ops differ");
        prop_assert!(sa.windows(2).all(|w| w[0] == w[1]), "A diverged");
        prop_assert!(sb.windows(2).all(|w| w[0] == w[1]), "B diverged");
        prop_assert_eq!(&sa[0], &sb[0], "A and B disagree");
    }
}

macro_rules! for_both {
    ($($f:ident),* $(,)?) => {
        mod alt_a { use super::*; $( #[test] fn $f() { super::$f::<AltA>(); } )* }
        mod alt_b { use super::*; $( #[test] fn $f() { super::$f::<AltB>(); } )* }
    };
}

for_both!(
    s01_offline_writes_then_reconnect,
    s02_different_fields_both_kept,
    s03_same_field_lww_with_visible_conflict,
    s03_equal_stamps_break_ties_by_device,
    s04_delete_then_restore_equals_before,
    s05_edit_unseen_by_delete_resurrects_and_notifies,
    s11_index_rebuild_is_identical,
    foreign_circle_messages_are_refused,
);
