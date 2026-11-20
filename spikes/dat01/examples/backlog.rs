//! DAT-01 S-08. `cargo run -p regia-spike-dat01 --example backlog --release -- [reps] [ops]`
//!
//! 5 clients, `ops` offline operations each (default 1000), seeds `1..=reps` (default 10). One
//! raw line per run, then the verdict against the thresholds frozen in the experiment card:
//! p95 < 5000 ms, bytes <= 10x payload, no loss, no leak. Times are in-process CPU, no network.

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::env;

use regia_spike_dat01::{
    alt_a::AltA,
    alt_b::AltB,
    alt_c::AltC,
    backlog::{Outcome, run},
};

const P95_MS: f64 = 5000.0;
const MAX_RATIO: f64 = 10.0;

fn report(name: &str, runs: &[Outcome]) -> bool {
    let mut times: Vec<f64> = runs.iter().map(|o| o.sync_ms).collect();
    times.sort_by(f64::total_cmp);
    let p95 = times[((times.len() as f64 * 0.95).ceil() as usize).max(1) - 1];
    let ratio = runs
        .iter()
        .map(|o| o.bytes as f64 / o.payload as f64)
        .fold(0.0, f64::max);
    let clean = runs.iter().all(|o| o.converged && o.lost == 0 && !o.leaked);
    let ok = p95 < P95_MS && ratio <= MAX_RATIO && clean;
    println!(
        "{name}: p95={p95:.0}ms max_bytes/payload={ratio:.2} clean={clean} -> {}",
        if ok { "PASS" } else { "FAIL" }
    );
    ok
}

fn main() {
    let reps: u64 = env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(10);
    let ops: usize = env::args()
        .nth(2)
        .and_then(|a| a.parse().ok())
        .unwrap_or(1000);
    println!("reps={reps} ops_per_client={ops}");
    let mut all = true;
    for (name, f) in [
        (
            "A",
            run::<AltA> as fn(u64, usize) -> Result<Outcome, String>,
        ),
        ("B", run::<AltB>),
        ("C", run::<AltC>),
    ] {
        let runs: Vec<Outcome> = (1..=reps)
            .map(|seed| {
                let o = f(seed, ops).unwrap();
                println!("{name} seed={seed} {o:?}");
                o
            })
            .collect();
        all &= report(name, &runs);
    }
    assert!(all, "S-08 threshold missed");
}
