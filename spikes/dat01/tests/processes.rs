//! DAT-01 increment 2: S-07 with a real process kill, S-12 with concurrent processes.
//!
//! A child process (`src/bin/writer.rs`) writes to a file-backed store and is killed at a
//! pseudo-random moment. The invariants hold wherever the kill lands, so what the test asserts is
//! deterministic even though the kill point is not.
//!
//! Limits: `Child::kill` is SIGKILL on the CI image. That kills the process, not the machine: it
//! does not test what the OS had not yet written to disk (a power cut).

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::{
    io::{BufRead as _, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use regia_spike_dat01::{Replica, Snapshot, alt_a::AltA, alt_b::AltB};

const WRITER: &str = env!("CARGO_BIN_EXE_writer");

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("regia-dat01-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn spawn(alt: &str, store: &Path, tag: &str, n: u64) -> Child {
    Command::new(WRITER)
        .args([alt, store.to_str().unwrap(), tag, &n.to_string()])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

/// Runs a writer for `ms`, kills it, and returns how many ops it acknowledged.
fn kill_after(alt: &str, store: &Path, ms: u64) -> usize {
    let mut child = spawn(alt, store, "k", 1_000_000);
    let out = BufReader::new(child.stdout.take().unwrap());
    let counter = thread::spawn(move || out.lines().count());
    thread::sleep(Duration::from_millis(ms));
    child.kill().unwrap();
    child.wait().unwrap();
    counter.join().unwrap()
}

/// Syncs `r` with a fresh in-memory replica of the same kind; returns the peer's snapshot.
fn resync<R: Replica>(r: &mut R) -> Snapshot {
    let mut peer = R::new(9, &[0]);
    let (mut s1, mut s2) = (R::Sync::default(), R::Sync::default());
    for _ in 0..64 {
        let (m1, m2) = (r.generate(0, &mut s1), peer.generate(0, &mut s2));
        if m1.is_none() && m2.is_none() {
            return peer.snapshot(0);
        }
        if let Some(m) = m1 {
            peer.receive(0, &mut s2, &m).unwrap();
        }
        if let Some(m) = m2 {
            r.receive(0, &mut s1, &m).unwrap();
        }
    }
    panic!("session did not settle");
}

/// Reopens the store and checks it is whole. Returns the number of objects.
fn reopen(alt: &str, store: &Path) -> (usize, Snapshot) {
    let (snap, peer_snap) = if alt == "b" {
        let mut r = AltB::open(0, &[0], store).unwrap();
        assert!(r.integrity_ok(), "sqlite integrity_check failed");
        let ops = r.vv(0).get(&0).copied().unwrap_or(0);
        let snap = r.snapshot(0);
        // One op per object here, so a hole or a half-written object shows as a mismatch.
        assert_eq!(usize::try_from(ops).unwrap(), snap.objects.len());
        let peer = resync(&mut r);
        (snap, peer)
    } else {
        let mut r = AltA::new(0, &[0]);
        if store.exists() {
            r.load_circle(0, &std::fs::read(store).unwrap()).unwrap();
        }
        let snap = r.snapshot(0);
        let peer = resync(&mut r);
        (snap, peer)
    };
    assert!(
        snap.objects
            .values()
            .all(|o| o.fields.contains_key("title")),
        "torn object: no title"
    );
    assert_eq!(
        snap, peer_snap,
        "a fresh peer did not converge with the store"
    );
    (snap.objects.len(), snap)
}

/// Tiny deterministic generator for kill delays, 20..240 ms.
fn delays(seed: u64, n: usize) -> Vec<u64> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            20 + (x >> 33) % 220
        })
        .collect()
}

fn s07_kill(alt: &str) {
    let mut total = 0;
    for (round, ms) in delays(7, 12).into_iter().enumerate() {
        let d = dir(&format!("s07-{alt}-{round}"));
        let store = d.join("store");
        let acked = kill_after(alt, &store, ms);
        total += acked;
        let (present, _) = reopen(alt, &store);
        // Everything acknowledged is there; at most the op in flight at the kill is too.
        assert!(
            (acked..=acked + 1).contains(&present),
            "round {round}: acked {acked}, present {present}"
        );
        let _ = std::fs::remove_dir_all(d);
    }
    assert!(
        total > 0,
        "no writer ever acknowledged an op: the test proves nothing"
    );
}

fn s12_concurrent(alt: &str) {
    let d = dir(&format!("s12-{alt}"));
    let store = d.join("store");
    let n = 60;
    let mut kids: Vec<Child> = ["p", "q"]
        .iter()
        .map(|t| spawn(alt, &store, t, n))
        .collect();
    for k in &mut kids {
        assert!(k.wait().unwrap().success(), "a writer failed");
    }
    let (present, snap) = reopen(alt, &store);
    assert_eq!(present, 2 * usize::try_from(n).unwrap(), "lost update");
    assert!(snap.conflicts.is_empty());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn s07_b_killed_process_leaves_a_whole_store() {
    s07_kill("b");
}

#[test]
fn s07_a_killed_process_leaves_a_whole_store() {
    s07_kill("a");
}

#[test]
fn s12_b_two_processes_lose_nothing() {
    s12_concurrent("b");
}

#[test]
fn s12_a_two_processes_lose_nothing() {
    s12_concurrent("a");
}
