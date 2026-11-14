//! Child process of the S-07 (kill) and S-12 (concurrent processes) tests.
//!
//! `writer <a|b> <store> <tag> <n>` creates `n` objects `<tag>-<i>` in circle 0 of a file-backed
//! store, printing one line per op *after* it is durable. The test kills it at a random point and
//! reopens the store: everything printed must be there, and nothing torn.

#![allow(clippy::expect_used)]

use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::Write as _,
    path::Path,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use regia_spike_dat01::{Op, Replica, alt_a::AltA, alt_b::AltB};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0))
}

fn create(obj: String) -> Op {
    Op::Create {
        fields: BTreeMap::from([("title".to_owned(), format!("title of {obj}"))]),
        obj,
    }
}

/// A: the store is one Automerge file. Writers take an exclusive lock, read the file, apply the
/// op, write a temp file, fsync it and rename it over the store, so a reader sees the old file or
/// the new one, never half of it.
///
/// `ponytail:` the directory is not fsynced after the rename. A process kill is covered; a power
/// cut is not. Add the directory fsync if ADR-002 picks Automerge.
fn write_a(path: &Path, op: Op) -> Result<(), String> {
    let e = |e: std::io::Error| e.to_string();
    let lock = File::create(path.with_extension("lock")).map_err(e)?;
    lock.lock().map_err(e)?;
    let mut r = AltA::new(0, &[0]);
    if path.exists() {
        r.load_circle(0, &fs::read(path).map_err(e)?)?;
    }
    r.apply_local(0, op, now_ms())?;
    let tmp = path.with_extension("tmp");
    let mut f = File::create(&tmp).map_err(e)?;
    f.write_all(&r.save_circle(0)?).map_err(e)?;
    f.sync_all().map_err(e)?;
    fs::rename(&tmp, path).map_err(e)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let [_, alt, path, tag, n] = &args[..] else {
        eprintln!("usage: writer <a|b> <store> <tag> <n>");
        return ExitCode::from(2);
    };
    let (path, n) = (Path::new(path), n.parse::<u64>().unwrap_or(0));
    let mut b = (alt == "b").then(|| AltB::open(0, &[0], path).expect("open store"));
    let mut out = std::io::stdout().lock();
    for i in 0..n {
        let op = create(format!("{tag}-{i}"));
        let done = match b.as_mut() {
            Some(b) => b.apply_local(0, op, now_ms()),
            None => write_a(path, op),
        };
        if let Err(e) = done {
            eprintln!("op {i}: {e}");
            return ExitCode::FAILURE;
        }
        // The line is the acknowledgement: it is written only once the op is durable.
        if writeln!(out, "ok {i}").and_then(|()| out.flush()).is_err() {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
