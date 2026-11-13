//! `cargo run -p regia-domain --features synthetic --example dat01 -- <seed> <small|large>`
//! prints one JSON `Envelope` per line (see `regia_domain::synthetic`).

use std::{env, process::ExitCode};

use regia_domain::synthetic::{LARGE, SMALL, generate};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let parsed = match args.as_slice() {
        [seed, profile] => seed.parse::<u64>().ok().zip(match profile.as_str() {
            "small" => Some(&SMALL),
            "large" => Some(&LARGE),
            _ => None,
        }),
        _ => None,
    };
    let Some((seed, profile)) = parsed else {
        eprintln!("usage: dat01 <seed: u64> <small|large>");
        return ExitCode::from(2);
    };
    eprintln!("dat01: seed={seed} profile={}", profile.name);
    for envelope in generate(seed, profile) {
        match serde_json::to_string(&envelope) {
            Ok(line) => println!("{line}"),
            Err(e) => {
                eprintln!("serialisation failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
