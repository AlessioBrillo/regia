//! `cargo run -p regia-domain --features synthetic --example query -- --store target/demo-store.jsonl --q "anna"`
//!
//! Read-only case-insensitive scan of the demo JSONL: prints matching objects and the
//! evidence attached to the field that matched. Objects outside the reader's circle and
//! objects labelled `restricted`/`secret` are withheld **before** matching. Invalid lines and
//! withheld objects are counted, never hidden. This is not an index and decides nothing
//! about ADR-011.

use std::{env, fmt::Write as _, fs, process::ExitCode};

use regia_domain::{
    CircleId, Envelope, Locator, ReadDecision, ReadView, Withheld, synthetic::search_envelopes,
};

struct Args {
    store: String,
    needle: String,
    circle: CircleId,
}

fn parse_args() -> Result<Args, String> {
    let mut store = None;
    let mut needle = None;
    let mut circle = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--store" => store = Some(args.next().ok_or("--store needs a value")?),
            "--q" => needle = Some(args.next().ok_or("--q needs a value")?),
            "--circle" => {
                let value = args.next().ok_or("--circle needs a value")?;
                circle = Some(
                    value
                        .parse::<CircleId>()
                        .map_err(|e| format!("--circle: {e}"))?,
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let store = store.ok_or("--store is required")?;
    let needle = needle.ok_or("--q is required")?;
    // Fail closed: without a circle the query renders nothing.
    let circle = circle.ok_or("--circle is required")?;
    Ok(Args {
        store,
        needle,
        circle,
    })
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
            eprintln!("usage: query --store <file.jsonl> --circle <uuid-v7> --q <text>");
            return ExitCode::from(2);
        }
    };

    let view = ReadView::owner(args.circle);
    let jsonl = match fs::read_to_string(&args.store) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("query: cannot read {}: {e}", args.store);
            return ExitCode::FAILURE;
        }
    };

    // Filter before matching: a withheld object must not affect even the match count.
    let mut visible = Vec::new();
    let mut withheld_label = 0;
    let mut withheld_circle = 0;
    let mut refused = 0;
    for line in jsonl.lines() {
        match serde_json::from_str::<Envelope>(line) {
            Ok(envelope) => match view.evaluate(&envelope) {
                ReadDecision::Show => visible.push(envelope),
                ReadDecision::Withheld(Withheld::Label) => withheld_label += 1,
                ReadDecision::Withheld(Withheld::Circle) => withheld_circle += 1,
            },
            Err(_) => refused += 1,
        }
    }
    let report = search_envelopes(visible.iter().map(Some), &args.needle);

    let mut output = String::new();
    for m in &report.matches {
        let _ = writeln!(output, "id: {}", m.id);
        match m.area {
            Some(area) => {
                let _ = writeln!(output, "area: {area}");
            }
            None => output.push_str("area: not filed yet\n"),
        }
        let _ = writeln!(output, "text: {}", m.text);
        if m.sources.is_empty() {
            let _ = writeln!(
                output,
                "evidence: unsourced ({})",
                m.unsourced.as_deref().unwrap_or("no reason")
            );
        }
        for source in &m.sources {
            let _ = writeln!(
                output,
                "source: {}  locator: {}  captured_at: {}",
                source.source,
                format_locator(source.locator),
                source.captured_at
            );
        }
        output.push('\n');
    }
    let _ = writeln!(
        output,
        "matches: {}  withheld: {} (label: {}, circle: {})  refused lines: {}",
        report.matches.len(),
        withheld_label + withheld_circle,
        withheld_label,
        withheld_circle,
        refused
    );

    // Last line of defence: never print a canary, even if a future change breaks the filter.
    if output.contains("CANARY-") {
        eprintln!("query: canary found in rendered output; refusing to print");
        return ExitCode::FAILURE;
    }
    print!("{output}");
    ExitCode::SUCCESS
}
