//! DAT-01 increment 1 benchmark. `cargo run -p regia-spike-dat01 --example bench --release -- [reps]`
//!
//! For each alternative, dataset profile and client count: populate the dataset offline, edit,
//! sync, then force one conflict per shared object. Prints one raw line per run, then a summary
//! (median and p95 over the seeds `1..=reps`). Everything is synthetic.
//!
//! Wall times are CPU time of in-process work. There is no network: "convergence" here is the
//! time `sync_all` needs, a lower bound, not the 5 s target of the owner (which needs real links).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::{collections::BTreeMap, env, time::Instant};

use regia_domain::synthetic::{LARGE, Profile, SMALL};
use regia_spike_dat01::{
    Op, Replica,
    alt_a::AltA,
    alt_b::AltB,
    data::Dataset,
    sim::Sim,
};

#[derive(Debug, Default, Clone)]
struct Run {
    write_us: Vec<f64>,
    sync_ms: f64,
    resync_ms: f64,
    bytes: usize,
    messages: usize,
    storage_per_op: f64,
    rebuild_ms: f64,
    conflicts: usize,
    forced: usize,
    intent_pct: f64,
    ops: usize,
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    v[(((v.len() - 1) as f64) * p).round() as usize]
}

fn run<R: Replica>(seed: u64, profile: &Profile, n: u32) -> Run {
    let ds = Dataset::load(seed, profile);
    let scope = (0..n).map(|d| ds.circles_of(d)).collect();
    let mut sim = Sim::<R>::new(scope);
    let mut r = Run::default();
    let mut created = Vec::new();

    for (i, item) in ds.items.iter().enumerate() {
        let m = sim.members(item.circle);
        if m.is_empty() {
            continue;
        }
        let d = m[i % m.len()];
        let f = BTreeMap::from([("title".to_owned(), item.title.clone()), ("status".to_owned(), "open".to_owned())]);
        let t = Instant::now();
        sim.op(d, item.circle, Op::Create { obj: item.id.clone(), fields: f }).unwrap();
        r.write_us.push(t.elapsed().as_secs_f64() * 1e6);
        created.push((item.clone(), d));
        r.ops += 1;
    }
    for (item, d) in created.iter().step_by(10) {
        let t = Instant::now();
        let op = Op::Set { obj: item.id.clone(), field: "status".into(), value: "done".into() };
        sim.op(*d, item.circle, op).unwrap();
        r.write_us.push(t.elapsed().as_secs_f64() * 1e6);
        r.ops += 1;
    }

    let t = Instant::now();
    sim.sync_all().unwrap();
    r.sync_ms = t.elapsed().as_secs_f64() * 1e3;

    // Intent: every created object is present, with its title, on every member of its circle.
    let mut present = 0;
    let mut expected = 0;
    for (item, _) in &created {
        for d in sim.members(item.circle) {
            expected += 1;
            let snap = sim.clients[d as usize].snapshot(item.circle);
            present += usize::from(snap.objects.get(&item.id).is_some_and(|v| v.fields.get("title") == Some(&item.title)));
        }
    }

    // Force one same-field conflict on up to 200 objects shared by two devices.
    let t_conflict = Instant::now();
    let shared: Vec<_> = created.iter().filter(|(i, _)| sim.members(i.circle).len() >= 2).take(200).collect();
    for (item, _) in shared {
        let m = sim.members(item.circle);
        let (a, b) = (m[0], m[1]);
        sim.op(a, item.circle, Op::Set { obj: item.id.clone(), field: "title".into(), value: "A".into() }).unwrap();
        sim.op(b, item.circle, Op::Set { obj: item.id.clone(), field: "title".into(), value: "B".into() }).unwrap();
        r.forced += 1;
        r.ops += 2;
    }
    sim.sync_all().unwrap();
    r.resync_ms = t_conflict.elapsed().as_secs_f64() * 1e3;
    r.conflicts = (0..ds.circles)
        .filter_map(|c| sim.members(c).first().copied().map(|d| sim.clients[d as usize].snapshot(c).conflicts.len()))
        .sum();

    r.bytes = sim.bytes;
    r.messages = sim.messages;
    let total_storage: usize = sim.clients.iter_mut().map(|c| c.storage_bytes()).sum();
    r.storage_per_op = total_storage as f64 / r.ops.max(1) as f64;
    let t = Instant::now();
    sim.clients[0].rebuild_index();
    r.rebuild_ms = t.elapsed().as_secs_f64() * 1e3;
    r.intent_pct = 100.0 * present as f64 / expected.max(1) as f64;
    r
}

fn loc(src: &str) -> usize {
    src.lines().filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with("//")).count()
}

fn main() {
    let reps: u64 = env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(10);
    let only_profile = env::args().nth(2);
    let only_n: Option<u32> = env::args().nth(3).and_then(|a| a.parse().ok());
    println!("reps={reps} (seeds 1..={reps}) profile={only_profile:?} clients={only_n:?}; times are in-process CPU, no network");
    println!("adapter LOC (non-blank, non-comment): A={} B={}", loc(include_str!("../src/alt_a.rs")), loc(include_str!("../src/alt_b.rs")));
    println!("\nRAW alt profile clients seed write_p50_us write_p95_us sync_ms resync_ms bytes messages storage_per_op rebuild_ms conflicts forced intent_pct");
    let mut all: BTreeMap<(String, &str, u32), Vec<Run>> = BTreeMap::new();
    for (name, profile) in [("small", &SMALL), ("large", &LARGE)].into_iter().filter(|(n, _)| only_profile.as_deref().is_none_or(|p| p == *n)) {
        for n in [2u32, 5].into_iter().filter(|n| only_n.is_none_or(|o| o == *n)) {
            for seed in 1..=reps {
                for alt in ["A", "B"] {
                    let r = if alt == "A" { run::<AltA>(seed, profile, n) } else { run::<AltB>(seed, profile, n) };
                    let (mut w1, mut w2) = (r.write_us.clone(), r.write_us.clone());
                    println!(
                        "RAW {alt} {name} {n} {seed} {:.1} {:.1} {:.1} {:.1} {} {} {:.0} {:.2} {} {} {:.2}",
                        pct(&mut w1, 0.5), pct(&mut w2, 0.95), r.sync_ms, r.resync_ms, r.bytes, r.messages,
                        r.storage_per_op, r.rebuild_ms, r.conflicts, r.forced, r.intent_pct
                    );
                    all.entry((alt.to_owned(), name, n)).or_default().push(r);
                }
            }
        }
    }
    println!("\nSUMMARY (median / p95 over seeds)");
    println!("alt profile clients | write p50 us | write p95 us | sync ms med/p95 | resync ms med | bytes med | storage B/op | rebuild ms | conflicts/forced | intent %");
    for ((alt, name, n), runs) in &all {
        let col = |f: &dyn Fn(&Run) -> f64, p: f64| pct(&mut runs.iter().map(f).collect::<Vec<_>>(), p);
        let mut writes: Vec<f64> = runs.iter().flat_map(|r| r.write_us.iter().copied()).collect();
        println!(
            "{alt} {name} {n} | {:.0} | {:.0} | {:.0}/{:.0} | {:.0} | {:.0} | {:.0} | {:.1} | {:.0}/{:.0} | {:.2}",
            pct(&mut writes, 0.5), pct(&mut writes, 0.95),
            col(&|r| r.sync_ms, 0.5), col(&|r| r.sync_ms, 0.95), col(&|r| r.resync_ms, 0.5),
            col(&|r| r.bytes as f64, 0.5), col(&|r| r.storage_per_op, 0.5), col(&|r| r.rebuild_ms, 0.5),
            col(&|r| r.conflicts as f64, 0.5), col(&|r| r.forced as f64, 0.5), col(&|r| r.intent_pct, 0.0),
        );
    }
}
