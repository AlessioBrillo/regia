//! `cargo run -p regia-domain --features synthetic --example query -- --store target/demo-store.jsonl --q "anna"`
//!
//! Read-only case-insensitive scan of the demo JSONL: prints matching objects and the
//! evidence attached to the field that matched. Invalid lines are counted, never hidden.
//! This is not an index and decides nothing about ADR-011.

use std::{env, fs, process::ExitCode};

use regia_domain::{Envelope, Locator, synthetic::search_envelopes};

struct Args {
    store: String,
    needle: String,
}

fn parse_args() -> Result<Args, String> {
    let mut store = None;
    let mut needle = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--store" => store = Some(args.next().ok_or("--store needs a value")?),
            "--q" => needle = Some(args.next().ok_or("--q needs a value")?),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let store = store.ok_or("--store is required")?;
    let needle = needle.ok_or("--q is required")?;
    Ok(Args { store, needle })
}

fn format_locator(locator: Option<Locator>) -> String {
    match locator {
        Some(Locator::Page(page)) => format!("page {page}"),
        Some(Locator::CharSpan { start, end }) => format!("chars {start}..{end}"),
        Some(_) => "unknown".to_owned(),
        None => "none".to_owned(),
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("query: {e}");
            eprintln!("usage: query --store <file.jsonl> --q <text>");
            return ExitCode::from(2);
        }
    };

    let jsonl = match fs::read_to_string(&args.store) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("query: cannot read {}: {e}", args.store);
            return ExitCode::FAILURE;
        }
    };
    let parsed: Vec<Option<Envelope>> = jsonl
        .lines()
        .map(|line| serde_json::from_str::<Envelope>(line).ok())
        .collect();
    let report = search_envelopes(parsed.iter().map(Option::as_ref), &args.needle);

    for m in &report.matches {
        println!("id: {}", m.id);
        match m.area {
            Some(area) => println!("area: {area}"),
            None => println!("area: not filed yet"),
        }
        println!("text: {}", m.text);
        if m.sources.is_empty() {
            println!(
                "evidence: unsourced ({})",
                m.unsourced.as_deref().unwrap_or("no reason")
            );
        }
        for source in &m.sources {
            println!(
                "source: {}  locator: {}  captured_at: {}",
                source.source,
                format_locator(source.locator),
                source.captured_at
            );
        }
        println!();
    }
    println!(
        "matches: {}  refused lines: {}",
        report.matches.len(),
        report.refused
    );
    ExitCode::SUCCESS
}
