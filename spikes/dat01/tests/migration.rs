//! DAT-01 increment 3, S-06: schema migration v0 -> v1 while clients are mid-sync.
//!
//! v1 is v0 plus the optional field `area`. Rules frozen in the experiment card before any run:
//! a v0 replica keeps and forwards `area` unchanged, flags the object "update the app" and
//! refuses to write `area`; nothing is lost or unreadable; the canary scan stays clean.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use regia_spike_dat01::{Op, Replica, Schema, alt_a::AltA, alt_b::AltB, sim::Sim};

const OBJ: &str = "o1";

/// Devices 0 and 2 run v1, device 1 runs v0. Circle 0 is shared; circle 1 holds device 2 only.
fn world<R: Replica>() -> Sim<R> {
    let mut sim = Sim::<R>::new(vec![
        BTreeSet::from([0]),
        BTreeSet::from([0]),
        BTreeSet::from([0, 1]),
    ]);
    for (d, s) in [(0, Schema::V1), (1, Schema::V0), (2, Schema::V1)] {
        sim.clients[d].set_schema(s);
    }
    sim
}

fn create<R: Replica>(sim: &mut Sim<R>, d: u32, circle: usize, obj: &str, title: &str, area: bool) {
    let mut fields = BTreeMap::from([("title".to_owned(), title.to_owned())]);
    if area {
        fields.insert("area".to_owned(), "work".to_owned());
    }
    let op = Op::Create {
        obj: obj.to_owned(),
        fields,
    };
    sim.op(d, circle, op).unwrap();
}

fn set<R: Replica>(sim: &mut Sim<R>, d: u32, field: &str, value: &str) -> Result<(), String> {
    let op = Op::Set {
        obj: OBJ.to_owned(),
        field: field.to_owned(),
        value: value.to_owned(),
    };
    sim.op(d, 0, op)
}

fn field<R: Replica>(sim: &mut Sim<R>, d: u32, name: &str) -> Option<String> {
    sim.clients[d as usize].snapshot(0).objects[OBJ]
        .fields
        .get(name)
        .cloned()
}

/// v0 shows the object and flags it; v1 sees `area`; v0 cannot write it; no loss.
fn s06_v0_flags_and_refuses<R: Replica>() {
    let mut sim = world::<R>();
    create(&mut sim, 0, 0, OBJ, "t", true);
    sim.sync_all().unwrap();

    let snap = sim.clients[1].snapshot(0);
    assert_eq!(snap.objects[OBJ].fields["title"], "t", "object unreadable");
    assert!(!snap.objects[OBJ].fields.contains_key("area"));
    assert!(snap.needs_update.contains(OBJ), "not flagged");
    assert_eq!(field(&mut sim, 0, "area").as_deref(), Some("work"));
    assert!(sim.clients[0].snapshot(0).needs_update.is_empty());

    assert!(set(&mut sim, 1, "area", "home").is_err(), "v0 wrote area");
    let fields = BTreeMap::from([
        ("title".to_owned(), "x".to_owned()),
        ("area".into(), "w".into()),
    ]);
    let op = Op::Create {
        obj: "o2".to_owned(),
        fields,
    };
    assert!(sim.op(1, 0, op).is_err(), "v0 created with area");
}

/// `area` written on v1 crosses a v0 device untouched, even while v0 edits other fields, and
/// the v1 replicas converge on the same visible state.
fn s06_v0_forwards_area_unchanged<R: Replica>() {
    let mut sim = world::<R>();
    create(&mut sim, 0, 0, OBJ, "t", true);
    sim.sync_pair(0, 1).unwrap();
    set(&mut sim, 1, "title", "edited by v0").unwrap();
    // Device 2 never talks to device 0: everything it learns about `area` comes through v0.
    sim.set_online(0, false);
    sim.sync_all().unwrap();
    assert_eq!(field(&mut sim, 2, "area").as_deref(), Some("work"));
    assert_eq!(field(&mut sim, 2, "title").as_deref(), Some("edited by v0"));
    sim.set_online(0, true);
    sim.sync_all().unwrap();
    assert_eq!(sim.clients[0].snapshot(0), sim.clients[2].snapshot(0));
    assert_eq!(field(&mut sim, 0, "area").as_deref(), Some("work"));
}

/// Concurrent edits across the version boundary: v1 sets `area`, v0 sets `title`. Both survive.
fn s06_mixed_concurrent_edits_both_kept<R: Replica>() {
    let mut sim = world::<R>();
    create(&mut sim, 0, 0, OBJ, "t", true);
    sim.sync_all().unwrap();
    set(&mut sim, 0, "area", "home").unwrap();
    set(&mut sim, 1, "title", "v0 title").unwrap();
    sim.sync_all().unwrap();
    for d in [0, 2] {
        assert_eq!(field(&mut sim, d, "area").as_deref(), Some("home"));
        assert_eq!(field(&mut sim, d, "title").as_deref(), Some("v0 title"));
    }
    assert_eq!(field(&mut sim, 1, "title").as_deref(), Some("v0 title"));
    assert!(sim.clients[1].snapshot(0).needs_update.contains(OBJ));
    assert_eq!(sim.clients[0].snapshot(0), sim.clients[2].snapshot(0));
}

/// A conflict on `area` is visible to v1 and not shown to v0, which only flags the object.
fn s06_area_conflict_is_hidden_from_v0_only<R: Replica>() {
    let mut sim = world::<R>();
    create(&mut sim, 0, 0, OBJ, "t", true);
    sim.sync_all().unwrap();
    set(&mut sim, 0, "area", "a").unwrap();
    set(&mut sim, 2, "area", "b").unwrap();
    sim.sync_all().unwrap();
    let v1 = sim.clients[0].snapshot(0);
    assert!(v1.conflicts.iter().any(|c| c.field == "area"), "v1 lost it");
    let v0 = sim.clients[1].snapshot(0);
    assert!(v0.conflicts.is_empty() && v0.needs_update.contains(OBJ));
    // Once v1 writes the field again, the conflict is gone everywhere.
    set(&mut sim, 0, "area", "a").unwrap();
    sim.sync_all().unwrap();
    assert!(sim.clients[2].snapshot(0).conflicts.is_empty());
}

/// Canary scan: a secret title in circle 1 (device 2 only) never reaches devices 0 and 1, and
/// the positive control shows device 2 does hold it.
fn s06_no_canary_leak<R: Replica>() {
    let mut sim = world::<R>();
    create(&mut sim, 2, 1, "c1", "CANARY-1-0", true);
    create(&mut sim, 0, 0, OBJ, "t", true);
    sim.sync_all().unwrap();
    assert!(sim.clients[2].dump_text().contains("CANARY-1-0"));
    for d in [0, 1] {
        assert!(!sim.clients[d].dump_text().contains("CANARY-1-0"));
    }
}

macro_rules! for_both {
    ($($f:ident),* $(,)?) => {
        mod alt_a { use super::*; $( #[test] fn $f() { super::$f::<AltA>(); } )* }
        mod alt_b { use super::*; $( #[test] fn $f() { super::$f::<AltB>(); } )* }
    };
}

for_both!(
    s06_v0_flags_and_refuses,
    s06_v0_forwards_area_unchanged,
    s06_mixed_concurrent_edits_both_kept,
    s06_area_conflict_is_hidden_from_v0_only,
    s06_no_canary_leak,
);
