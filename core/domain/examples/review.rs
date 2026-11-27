//! `cargo run -p regia-domain --features synthetic --example review -- --store <file.jsonl> --areas <file.json> --circle <uuid>`
//!
//! Demo-only review step: proposes an area for each visible, unfiled task with deterministic
//! keyword rules, asks a person to confirm, and rewrites the JSONL atomically. The rules are a
//! placeholder, not a model decision: ADR-008/009 and AI-01 increment 2 stay open.

use std::{
    collections::HashMap,
    env,
    fmt::Write as _,
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::ExitCode,
};

use regia_domain::{
    Area, AreaId, CircleId, Envelope, Object, ReadDecision, ReadView, Withheld,
    synthetic::{confirm_area, propose_area},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct AreaFile {
    areas: Vec<Area>,
}

struct Args {
    store: PathBuf,
    areas: PathBuf,
    circle: CircleId,
}

fn parse_args() -> Result<Args, String> {
    let mut store = None;
    let mut areas = None;
    let mut circle = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--store" => store = Some(PathBuf::from(args.next().ok_or("--store needs a value")?)),
            "--areas" => areas = Some(PathBuf::from(args.next().ok_or("--areas needs a value")?)),
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
    Ok(Args {
        store: store.ok_or("--store is required")?,
        areas: areas.ok_or("--areas is required")?,
        // Fail closed: without a circle the review renders nothing.
        circle: circle.ok_or("--circle is required")?,
    })
}

fn load_areas(path: &Path) -> Result<HashMap<&'static str, AreaId>, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file: AreaFile =
        serde_json::from_str(&text).map_err(|e| format!("invalid areas file: {e}"))?;
    let mut out = HashMap::new();
    for area in file.areas {
        if let Some(key) = match area.name.as_str() {
            "work" => Some("work"),
            "home" => Some("home"),
            "notes" => Some("notes"),
            _ => None,
        } {
            out.insert(key, area.id);
        }
    }
    Ok(out)
}

/// Illustrative keyword rules. They are deliberately transparent and local: no provider,
/// no model output and no new task family is introduced here.
fn proposed_key(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if ["lavoro", "work", "ufficio", "meeting", "anna"]
        .iter()
        .any(|word| lower.contains(word))
    {
        "work"
    } else if ["casa", "home", "spesa", "milk", "dog"]
        .iter()
        .any(|word| lower.contains(word))
    {
        "home"
    } else {
        "notes"
    }
}

fn render_proposal(
    id: &str,
    key: &str,
    area_id: AreaId,
    text: &str,
    evidence: Option<&regia_domain::SourceRef>,
) -> Result<String, String> {
    let mut out = String::new();
    let _ = writeln!(out, "{id} -> {key} ({area_id})\ntext: {text}");
    if let Some(source) = evidence {
        let _ = writeln!(
            out,
            "evidence: source {}  captured_at {}",
            source.source, source.captured_at
        );
    }
    if out.contains("CANARY-") {
        return Err("canary found in rendered output".to_owned());
    }
    Ok(out)
}

fn ask_confirmation(rendered: &str) -> Result<bool, String> {
    print!("{rendered}confirm [y/N]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|e| e.to_string())?;
    Ok(answer.trim().eq_ignore_ascii_case("y"))
}

fn rewrite_store(
    store: &Path,
    lines: &[String],
    replacements: &HashMap<usize, String>,
) -> Result<(), String> {
    // Untouched lines keep their original bytes and terminators (CRLF included).
    let updated = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            replacements
                .get(&i)
                .cloned()
                .unwrap_or_else(|| line.clone())
        })
        .collect::<String>();
    let temp = PathBuf::from(format!("{}.{}.tmp", store.display(), std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp)
        .map_err(|e| format!("cannot write {}: {e}", temp.display()))?;
    file.write_all(updated.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", temp.display()))?;
    file.sync_all()
        .map_err(|e| format!("cannot sync {}: {e}", temp.display()))?;
    drop(file);
    fs::rename(&temp, store).map_err(|e| {
        let _ = fs::remove_file(&temp);
        format!("cannot replace {}: {e}", store.display())
    })
}

enum LineOutcome {
    Refused,
    Withheld,
    NotApplicable,
    NoProposal,
    Skipped,
    Confirmed(String),
}

fn review_line(
    line: &str,
    view: &ReadView,
    areas: &HashMap<&'static str, AreaId>,
) -> Result<LineOutcome, String> {
    let Ok(envelope) = serde_json::from_str::<Envelope>(line) else {
        return Ok(LineOutcome::Refused);
    };
    match view.evaluate(&envelope) {
        ReadDecision::Show => {}
        ReadDecision::Withheld(Withheld::Label | Withheld::Circle) => {
            return Ok(LineOutcome::Withheld);
        }
    }
    let Object::Task(task) = envelope.object() else {
        return Ok(LineOutcome::NotApplicable);
    };
    if task.header.area.is_some() {
        return Ok(LineOutcome::NotApplicable);
    }
    let key = proposed_key(&task.title.value);
    let Some(area_id) = areas.get(key) else {
        return Ok(LineOutcome::NoProposal);
    };
    let proposed = propose_area(task, *area_id)
        .map_err(|e| format!("cannot propose for {}: {e}", task.header.id))?;
    let rendered = render_proposal(
        &task.header.id.to_string(),
        key,
        *area_id,
        &task.title.value,
        proposed.title.provenance.sources().next(),
    )?;
    if !ask_confirmation(&rendered)? {
        return Ok(LineOutcome::Skipped);
    }
    let chosen = confirm_area(task, *area_id, "chosen by user")
        .map_err(|e| format!("cannot confirm for {}: {e}", task.header.id))?;
    let envelope =
        Envelope::new(Object::Task(chosen)).map_err(|e| format!("invalid object: {e}"))?;
    let line =
        serde_json::to_string(&envelope).map_err(|e| format!("serialisation failed: {e}"))?;
    Ok(LineOutcome::Confirmed(line))
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("review: {e}");
            eprintln!("usage: review --store <file.jsonl> --areas <file.json> --circle <uuid-v7>");
            return ExitCode::from(2);
        }
    };
    let areas = match load_areas(&args.areas) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("review: {e}");
            return ExitCode::FAILURE;
        }
    };
    let lines = match fs::read_to_string(&args.store) {
        Ok(s) => s
            .split_inclusive('\n')
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("review: cannot read {}: {e}", args.store.display());
            return ExitCode::FAILURE;
        }
    };

    let view = ReadView::owner(args.circle);
    let mut replacements = HashMap::new();
    let (mut confirmed, mut skipped, mut withheld, mut refused, mut no_proposal) = (0, 0, 0, 0, 0);

    for (index, line) in lines.iter().enumerate() {
        let outcome = match review_line(line, &view, &areas) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("review: {e}");
                return ExitCode::FAILURE;
            }
        };
        match outcome {
            LineOutcome::Refused => refused += 1,
            LineOutcome::Withheld => withheld += 1,
            LineOutcome::NotApplicable => {}
            LineOutcome::NoProposal => no_proposal += 1,
            LineOutcome::Skipped => skipped += 1,
            LineOutcome::Confirmed(new_line) => {
                // Keep the terminator of the line being replaced.
                let ending = if line.ends_with("\r\n") {
                    "\r\n"
                } else if line.ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                replacements.insert(index, format!("{new_line}{ending}"));
                confirmed += 1;
            }
        }
    }

    // Never touch the store when nothing was confirmed: an empty store must stay empty.
    if !replacements.is_empty()
        && let Err(e) = rewrite_store(&args.store, &lines, &replacements)
    {
        eprintln!("review: {e}");
        return ExitCode::FAILURE;
    }
    println!(
        "confirmed: {confirmed}  skipped: {skipped}  no proposal: {no_proposal}  withheld: {withheld}  refused: {refused}"
    );
    ExitCode::SUCCESS
}
