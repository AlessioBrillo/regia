//! DAT-01 increment 2: S-09 (member revoked while a device is offline) and S-10 (object moved to
//! another circle). Rules frozen before results in `docs/experiments/DAT-01-data-model-sync.md`.
//!
//! Every scenario runs against both alternatives through the [`Replica`] trait. Synthetic data only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use regia_spike_dat01::{
    Circle, Device, Op, Quarantined, Reason, Replica, alt_a::AltA, alt_b::AltB, alt_c::AltC,
    sim::Sim,
};

const OLD: Circle = 0;
const NEW: Circle = 1;

fn create<R: Replica>(sim: &mut Sim<R>, d: Device, c: Circle, obj: &str, title: &str) {
    let fields = BTreeMap::from([
        ("title".to_owned(), title.to_owned()),
        ("status".to_owned(), "open".to_owned()),
        // Both sides of the hybrid: alternate by the last byte of the id.
        (
            "kind".to_owned(),
            if obj.bytes().last().unwrap_or(0).is_multiple_of(2) {
                "document"
            } else {
                "task"
            }
            .to_owned(),
        ),
    ]);
    sim.op(
        d,
        c,
        Op::Create {
            obj: obj.into(),
            fields,
        },
    )
    .unwrap();
}

fn set<R: Replica>(
    sim: &mut Sim<R>,
    d: Device,
    c: Circle,
    obj: &str,
    field: &str,
    value: &str,
) -> Result<(), String> {
    sim.op(
        d,
        c,
        Op::Set {
            obj: obj.into(),
            field: field.into(),
            value: value.into(),
        },
    )
}

fn q(obj: &str, author: Device, reason: Reason) -> Quarantined {
    Quarantined {
        obj: obj.into(),
        author,
        reason,
    }
}

/// Devices 0 (admin), 1 and 2, all in circle `OLD`; object "o" titled "t0" synced everywhere.
fn revocation_world<R: Replica>() -> Sim<R> {
    let mut sim = Sim::<R>::new(vec![BTreeSet::from([OLD]); 3]);
    create(&mut sim, 0, OLD, "o", "t0");
    sim.sync_all().unwrap();
    sim
}

// --- S-09 ----------------------------------------------------------------------------------------

/// Work done offline by a device that is revoked meanwhile is held aside for an admin: not
/// visible, not merged, and not lost.
fn s09_revoked_offline_edits_are_quarantined<R: Replica>() {
    let mut sim = revocation_world::<R>();
    sim.set_online(2, false);
    set(&mut sim, 2, OLD, "o", "title", "X-offline").unwrap();
    create(&mut sim, 2, OLD, "p", "p-offline");
    sim.revoke(0, OLD, 2).unwrap();
    sim.set_online(2, true);
    sim.sync_all().unwrap();
    let want = BTreeSet::from([q("o", 2, Reason::Revoked), q("p", 2, Reason::Revoked)]);
    let (s0, s1) = (sim.clients[0].snapshot(OLD), sim.clients[1].snapshot(OLD));
    assert_eq!(s0, s1, "members diverged");
    assert_eq!(s0.objects["o"].fields["title"], "t0");
    assert!(
        !s0.objects.contains_key("p"),
        "a quarantined create is visible"
    );
    assert_eq!(s0.quarantine, want);
    assert!(!sim.clients[0].index().contains_key(&(OLD, "p".into())));
    for d in [0, 1] {
        let text = sim.clients[d].dump_text();
        assert!(
            text.contains("X-offline") && text.contains("p-offline"),
            "device {d} lost the quarantined work"
        );
    }
}

/// What the admin had seen before the revocation stays; only what comes after is held.
fn s09_synced_before_revocation_stays<R: Replica>() {
    let mut sim = revocation_world::<R>();
    set(&mut sim, 2, OLD, "o", "title", "kept").unwrap();
    sim.sync_all().unwrap();
    sim.set_online(2, false);
    set(&mut sim, 2, OLD, "o", "status", "lost").unwrap();
    sim.revoke(0, OLD, 2).unwrap();
    sim.set_online(2, true);
    sim.sync_all().unwrap();
    let snap = sim.clients[1].snapshot(OLD);
    assert_eq!(snap.objects["o"].fields["title"], "kept");
    assert_eq!(snap.objects["o"].fields["status"], "open");
    assert_eq!(
        snap.quarantine,
        BTreeSet::from([q("o", 2, Reason::Revoked)])
    );
    assert_eq!(snap, sim.clients[0].snapshot(OLD));
}

/// An op that reached a peer before the revocation reached that peer is still held aside once
/// the revocation arrives: the cut is causal, so who heard it first does not matter.
fn s09_gossiped_before_revocation_still_quarantined<R: Replica>() {
    let mut sim = revocation_world::<R>();
    sim.set_online(0, false);
    set(&mut sim, 2, OLD, "o", "title", "gossip").unwrap();
    sim.sync_pair(1, 2).unwrap(); // device 1 now holds the edit; the admin has not seen it
    assert_eq!(
        sim.clients[1].snapshot(OLD).objects["o"].fields["title"],
        "gossip",
        "precondition: the edit was visible before the revocation"
    );
    sim.revoke(0, OLD, 2).unwrap();
    sim.set_online(0, true);
    sim.sync_all().unwrap();
    for d in [0, 1] {
        let snap = sim.clients[d].snapshot(OLD);
        assert_eq!(snap.objects["o"].fields["title"], "t0", "device {d}");
        assert_eq!(
            snap.quarantine,
            BTreeSet::from([q("o", 2, Reason::Revoked)])
        );
    }
}

/// A revoked device cannot get its work in by writing with a clock set in the past.
fn s09_backward_clock_cannot_bypass<R: Replica>() {
    let mut sim = revocation_world::<R>();
    sim.set_online(2, false);
    sim.op_at(
        2,
        OLD,
        Op::Set {
            obj: "o".into(),
            field: "title".into(),
            value: "backdated".into(),
        },
        1,
    )
    .unwrap();
    sim.revoke(0, OLD, 2).unwrap();
    sim.set_online(2, true);
    sim.sync_all().unwrap();
    let snap = sim.clients[0].snapshot(OLD);
    assert_eq!(snap.objects["o"].fields["title"], "t0");
    assert_eq!(
        snap.quarantine,
        BTreeSet::from([q("o", 2, Reason::Revoked)])
    );
}

/// After the revocation the member receives nothing: not the change, not the later data.
fn s09_no_later_data<R: Replica>() {
    let mut sim = revocation_world::<R>();
    sim.revoke(0, OLD, 2).unwrap();
    create(&mut sim, 0, OLD, "late", "CANARY-late");
    sim.sync_all().unwrap();
    assert!(
        !sim.clients[2].dump_text().contains("CANARY-late"),
        "the revoked device received data created after the revocation"
    );
    assert!(
        sim.clients[1].dump_text().contains("CANARY-late"),
        "positive control: a remaining member holds it"
    );
}

/// Finding for the ADRs. B keeps an edit that was built on a quarantined one, because its rule
/// is per op. Automerge cannot take a change out of history without the changes that depend on
/// it, so A holds those aside too.
fn dependent_setup<R: Replica>() -> Sim<R> {
    let mut sim = revocation_world::<R>();
    sim.set_online(0, false);
    set(&mut sim, 2, OLD, "o", "title", "m").unwrap();
    sim.sync_pair(1, 2).unwrap();
    set(&mut sim, 1, OLD, "o", "status", "built-on-m").unwrap();
    sim.revoke(0, OLD, 2).unwrap();
    sim.set_online(0, true);
    sim.sync_all().unwrap();
    sim
}

#[test]
fn s09_dependent_edit_stays_visible_on_b() {
    let mut sim = dependent_setup::<AltB>();
    let snap = sim.clients[0].snapshot(OLD);
    assert_eq!(snap.objects["o"].fields["title"], "t0");
    assert_eq!(snap.objects["o"].fields["status"], "built-on-m");
    assert_eq!(
        snap.quarantine,
        BTreeSet::from([q("o", 2, Reason::Revoked)])
    );
    assert_eq!(snap, sim.clients[1].snapshot(OLD));
}

#[test]
fn s09_dependent_edit_is_held_aside_on_a() {
    let mut sim = dependent_setup::<AltA>();
    let snap = sim.clients[0].snapshot(OLD);
    assert_eq!(snap.objects["o"].fields["title"], "t0");
    assert_eq!(
        snap.objects["o"].fields["status"], "open",
        "A has to hold aside what was built on a quarantined change"
    );
    assert_eq!(
        snap.quarantine,
        BTreeSet::from([q("o", 2, Reason::Revoked), q("o", 1, Reason::Dependent)])
    );
    assert_eq!(snap, sim.clients[1].snapshot(OLD));
    assert!(
        sim.clients[0].dump_text().contains("built-on-m"),
        "held aside, not lost"
    );
}

// --- S-10 ----------------------------------------------------------------------------------------

/// Devices 0 and 1 are in both circles, device 2 only in `OLD`, device 3 only in `NEW`.
fn move_world<R: Replica>() -> Sim<R> {
    Sim::new(vec![
        BTreeSet::from([OLD, NEW]),
        BTreeSet::from([OLD, NEW]),
        BTreeSet::from([OLD]),
        BTreeSet::from([NEW]),
    ])
}

fn mv(obj: &str) -> Op {
    Op::Move {
        obj: obj.into(),
        to: NEW,
    }
}

fn s10_move_leaves_no_residue_in_the_old_circle<R: Replica>() {
    let mut sim = move_world::<R>();
    create(&mut sim, 0, OLD, "o", "CANARY-move");
    sim.sync_all().unwrap();
    assert!(sim.clients[2].index().contains_key(&(OLD, "o".into())));
    sim.op(0, OLD, mv("o")).unwrap();
    sim.sync_all().unwrap();
    for d in [0, 1, 2] {
        let snap = sim.clients[d].snapshot(OLD);
        assert!(!snap.objects.contains_key("o"), "device {d} still shows it");
        assert!(
            !sim.clients[d].index().contains_key(&(OLD, "o".into())),
            "device {d} still indexes it in the old circle"
        );
    }
    for d in [0, 1, 3] {
        let snap = sim.clients[d].snapshot(NEW);
        assert_eq!(snap.objects["o"].fields["title"], "CANARY-move");
        assert_eq!(
            sim.clients[d].index()[&(NEW, "o".into())],
            "CANARY-move",
            "device {d}"
        );
    }
    // Downloaded copies cannot be recalled (T-02): the old circle's history keeps them, hidden.
    assert!(sim.clients[2].dump_text().contains("CANARY-move"));
    // Later changes in the new circle never reach a member of the old circle only.
    set(&mut sim, 1, NEW, "o", "title", "CANARY-after").unwrap();
    sim.sync_all().unwrap();
    assert!(!sim.clients[2].dump_text().contains("CANARY-after"));
    assert!(sim.clients[3].dump_text().contains("CANARY-after"));
    // The old circle refuses edits of a moved object.
    assert!(set(&mut sim, 1, OLD, "o", "title", "back").is_err());
    for d in [0, 1, 2] {
        let k = sim.clients[d].snapshot(OLD);
        assert_eq!(k, sim.clients[0].snapshot(OLD), "device {d} diverged");
    }
}

/// An edit made in the old circle by someone who had not seen the move is quarantined for the
/// mover and its author is told; it does not reappear in the old circle or travel to the new one.
fn s10_unseen_edit_in_old_circle_is_quarantined_and_notified<R: Replica>() {
    let mut sim = move_world::<R>();
    create(&mut sim, 0, OLD, "o", "t0");
    sim.sync_all().unwrap();
    sim.set_online(2, false);
    set(&mut sim, 2, OLD, "o", "title", "late-edit").unwrap();
    sim.op(0, OLD, mv("o")).unwrap();
    sim.set_online(2, true);
    sim.sync_all().unwrap();
    let want = BTreeSet::from([q("o", 2, Reason::MovedAway)]);
    for d in [0, 1, 2] {
        let snap = sim.clients[d].snapshot(OLD);
        assert!(!snap.objects.contains_key("o"), "device {d}");
        assert_eq!(snap.quarantine, want, "device {d}: the author must be told");
    }
    assert!(sim.clients[0].dump_text().contains("late-edit"), "not lost");
    for d in [0, 1, 3] {
        let snap = sim.clients[d].snapshot(NEW);
        assert_eq!(snap.objects["o"].fields["title"], "t0", "device {d}");
    }
    assert!(!sim.clients[3].dump_text().contains("late-edit"));
}

fn s10_move_with_open_conflicts_is_refused<R: Replica>() {
    let mut sim = move_world::<R>();
    create(&mut sim, 0, OLD, "o", "t0");
    sim.sync_all().unwrap();
    sim.set_online(1, false);
    set(&mut sim, 0, OLD, "o", "title", "a").unwrap();
    set(&mut sim, 1, OLD, "o", "title", "b").unwrap();
    sim.set_online(1, true);
    sim.sync_all().unwrap();
    assert_eq!(sim.clients[0].snapshot(OLD).conflicts.len(), 1);
    let err = sim.op(0, OLD, mv("o")).unwrap_err();
    assert!(err.contains("conflict"), "unexpected refusal: {err}");
    assert!(sim.clients[0].snapshot(OLD).objects.contains_key("o"));
    // Writing the field again resolves it, and then the move goes through.
    set(&mut sim, 0, OLD, "o", "title", "resolved").unwrap();
    sim.op(0, OLD, mv("o")).unwrap();
    sim.sync_all().unwrap();
    assert_eq!(
        sim.clients[3].snapshot(NEW).objects["o"].fields["title"],
        "resolved"
    );
}

/// The mover must hold the destination, and the destination must not already hold the object.
fn s10_move_needs_both_circles<R: Replica>() {
    let mut sim = move_world::<R>();
    create(&mut sim, 2, OLD, "o", "t0");
    assert!(sim.op(2, OLD, mv("o")).is_err(), "device 2 is not in NEW");
    assert!(sim.op(0, OLD, mv("missing")).is_err());
}

macro_rules! for_both {
    ($($f:ident),* $(,)?) => {
        mod alt_a { use super::*; $( #[test] fn $f() { super::$f::<AltA>(); } )* }
        mod alt_b { use super::*; $( #[test] fn $f() { super::$f::<AltB>(); } )* }
        mod alt_c { use super::*; $( #[test] fn $f() { super::$f::<AltC>(); } )* }
    };
}

for_both!(
    s09_revoked_offline_edits_are_quarantined,
    s09_synced_before_revocation_stays,
    s09_gossiped_before_revocation_still_quarantined,
    s09_backward_clock_cannot_bypass,
    s09_no_later_data,
    s10_move_leaves_no_residue_in_the_old_circle,
    s10_unseen_edit_in_old_circle_is_quarantined_and_notified,
    s10_move_with_open_conflicts_is_refused,
    s10_move_needs_both_circles,
);
