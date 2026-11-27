//! `cargo run -p regia-domain --features synthetic --example capture -- --text "..." --store target/demo-store.jsonl`
//!
//! Captures synthetic text as one JSON `Envelope` per line, appended to `--store`.
//! Without `--text`, the text is read from stdin. Demo only: no real data, no storage decision.

use std::{
    env,
    fs::OpenOptions,
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

use regia_domain::{Envelope, Object, Timestamp, synthetic::capture_text};
use time::OffsetDateTime;

struct Args {
    text: Option<String>,
    store: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut text = None;
    let mut store = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--text" => text = Some(args.next().ok_or("--text needs a value")?),
            "--store" => store = Some(PathBuf::from(args.next().ok_or("--store needs a value")?)),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let store = store.ok_or("--store is required")?;
    Ok(Args { text, store })
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("capture: {e}");
            eprintln!("usage: capture --store <file.jsonl> [--text <text>] (stdin if no --text)");
            return ExitCode::from(2);
        }
    };

    let Args { text: given, store } = args;
    let text = if let Some(t) = given {
        t
    } else {
        let mut s = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut s) {
            eprintln!("capture: cannot read stdin: {e}");
            return ExitCode::FAILURE;
        }
        s
    };
    let text = text.trim_end_matches(['\r', '\n']);

    let task = match capture_text(text, Timestamp(OffsetDateTime::now_utc())) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("capture: refused: {e}");
            return ExitCode::FAILURE;
        }
    };
    let id = task.header.id;
    let envelope = match Envelope::new(Object::Task(task)) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("capture: invalid object: {e}");
            return ExitCode::FAILURE;
        }
    };
    let line = match serde_json::to_string(&envelope) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("capture: serialisation failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut file = match OpenOptions::new().create(true).append(true).open(&store) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("capture: cannot open {}: {e}", store.display());
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = writeln!(file, "{line}") {
        eprintln!("capture: cannot append: {e}");
        return ExitCode::FAILURE;
    }
    println!("captured {id} -> {}", store.display());
    ExitCode::SUCCESS
}
