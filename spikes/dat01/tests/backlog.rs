//! S-08 with a small backlog so the gate stays fast; the 1000-op run is `examples/backlog.rs`.

#![allow(clippy::unwrap_used)]

use regia_spike_dat01::{Replica, alt_a::AltA, alt_b::AltB, alt_c::AltC, backlog::run};

fn small_backlog<R: Replica>() {
    for seed in 1..=3 {
        let o = run::<R>(seed, 40).unwrap();
        assert!(o.converged, "seed {seed} diverged");
        assert_eq!(o.lost, 0, "seed {seed} lost a write");
        assert!(!o.leaked, "seed {seed} leaked the canary");
        assert!(o.bytes > 0 && o.payload > 0);
    }
}

#[test]
fn s08_small_backlog_a() {
    small_backlog::<AltA>();
}

#[test]
fn s08_small_backlog_b() {
    small_backlog::<AltB>();
}

#[test]
fn s08_small_backlog_c() {
    small_backlog::<AltC>();
}
