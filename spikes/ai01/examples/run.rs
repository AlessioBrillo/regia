//! Runs one runtime over the golden set and prints the scoreboard of the frozen rules.
//!
//! `cargo run -p regia-spike-ai01 --release --example run -- --runtime r3:qwen2.5:3b [--reps 5] [--limit N] [--out raw.jsonl]`
//! Runtimes: `r0`, `r3:<ollama model>` (`OLLAMA_URL`), `r4:<mistral model>` (`MISTRAL_API_KEY`). Raw lines hold the
//! case id and the verdict, never the input text.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::type_complexity
)]

use std::{collections::BTreeMap, fmt::Write as _, io::Write as _, path::PathBuf};

use regia_spike_ai01::{
    Gateway, Golden, Outcome, llm,
    r0::R0,
    r3::Ollama,
    r4::Mistral,
    score::{Score, Verdict, score},
};
use serde_json::json;

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1))
        .cloned()
}

fn tag(v: &Verdict) -> &'static str {
    match v {
        Verdict::Pass => "pass",
        Verdict::Fail(_) => "fail",
        Verdict::Dangerous(_) => "dangerous",
    }
}

fn detail(v: &Verdict) -> &str {
    match v {
        Verdict::Pass => "",
        Verdict::Fail(m) | Verdict::Dangerous(m) => m,
    }
}

fn pct(sorted: &[u64], p: f64) -> u64 {
    sorted
        .get((((sorted.len().max(1) - 1) as f64) * p).round() as usize)
        .copied()
        .unwrap_or(0)
}

fn main() -> Result<(), String> {
    let spec = arg("--runtime").ok_or("--runtime r0|r3:<model>|r4:<model>")?;
    let reps: usize = arg("--reps").map_or(5, |s| s.parse().unwrap());
    let limit: usize = arg("--limit").map_or(usize::MAX, |s| s.parse().unwrap());
    let (kind, model) = spec.split_once(':').unwrap_or((spec.as_str(), ""));
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    let mut gw = Gateway::default();
    let probe = (kind == "r3")
        .then(|| Ollama::new(&ollama_url, model))
        .transpose()?;
    match kind {
        "r0" => gw.add(Box::new(R0)),
        "r3" => gw.add(Box::new(Ollama::new(&ollama_url, model)?)),
        "r4" => gw.add(Box::new(Mistral::new(model)?)),
        _ => return Err(format!("unknown runtime {kind}")),
    }
    let cloud = kind == "r4";
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/experiments/ai-01/golden/intent-v1.jsonl");
    let cases: Vec<Golden> = Golden::load(&golden)?.into_iter().take(limit).collect();
    let mut out = arg("--out")
        .map(|p| std::fs::File::create(p).map_err(|e| e.to_string()))
        .transpose()?;

    let mut runs: BTreeMap<String, Vec<Score>> = BTreeMap::new();
    let (mut lat, mut tokens, mut ram, mut version) = (vec![], (0u64, 0u64), 0u64, String::new());
    for (n, case) in cases.iter().enumerate() {
        for rep in 0..reps {
            let mut req = case.request()?;
            req.cloud_consent = cloud; // owner consent for synthetic input only; the router still refuses canary
            let (res, audit) = gw.run(&req, kind);
            let s = score(case, &res, |r| r != "r4");
            let (tin, tout) = llm::take_tokens();
            let ms = res.as_ref().map_or(0, |r| r.latency_ms);
            if ms > 0 {
                lat.push(ms);
            }
            tokens = (tokens.0 + tin, tokens.1 + tout);
            version.clone_from(&audit.version);
            if let Some(f) = out.as_mut() {
                let line = json!({"case": case.id, "rep": rep, "runtime": audit.runtime, "version": audit.version,
                    "prompt": llm::PROMPT_VERSION, "verdict": tag(&s.verdict), "detail": detail(&s.verdict),
                    "minor": s.minor, "latency_ms": ms, "route": audit.reason, "tokens_in": tin, "tokens_out": tout});
                writeln!(f, "{line}").map_err(|e| e.to_string())?;
            }
            runs.entry(case.id.clone()).or_default().push(s);
        }
        if n % 10 == 0 {
            ram = ram.max(probe.as_ref().and_then(Ollama::loaded_bytes).unwrap_or(0));
        }
    }
    ram = ram.max(probe.as_ref().and_then(Ollama::loaded_bytes).unwrap_or(0));

    let (mut by_cat, mut by_lang): (BTreeMap<_, (u32, u32)>, BTreeMap<_, (u32, u32)>) =
        Default::default();
    let (mut dangerous, mut dangerous_critical, mut minor, mut unstable) = (0, 0, 0, 0);
    let (mut clar_ok, mut clar_guess, mut clar_wrongly_asked) = (0, 0, 0);
    for c in &cases {
        let rs = &runs[&c.id];
        let passed = rs.iter().all(|s| s.verdict == Verdict::Pass);
        for key in [
            (&mut by_cat, c.category.clone()),
            (&mut by_lang, c.lang.clone()),
        ] {
            let e = key.0.entry(key.1).or_default();
            e.0 += u32::from(passed);
            e.1 += 1;
        }
        let d = rs
            .iter()
            .filter(|s| matches!(s.verdict, Verdict::Dangerous(_)))
            .count();
        dangerous += d;
        dangerous_critical += if c.critical { d } else { 0 };
        minor += rs.iter().map(|s| s.minor).sum::<usize>();
        unstable += usize::from(rs.iter().any(|s| tag(&s.verdict) != tag(&rs[0].verdict)));
        if c.expect.outcome == Outcome::Clarify {
            clar_ok += usize::from(passed);
            clar_guess += rs
                .iter()
                .filter(|s| detail(&s.verdict) == "guessed instead of asking")
                .count();
        } else {
            clar_wrongly_asked += rs
                .iter()
                .filter(|s| detail(&s.verdict).starts_with("outcome Clarify"))
                .count();
        }
    }
    lat.sort_unstable();
    let total = cases.len();
    let passed_all: u32 = by_cat.values().map(|v| v.0).sum();
    let mut s = String::new();
    let _ = writeln!(
        s,
        "runtime {kind}  version {version}  prompt {}",
        llm::PROMPT_VERSION
    );
    let _ = writeln!(
        s,
        "cases {total}  reps {reps}  (a case passes only if every rep passes)"
    );
    let _ = writeln!(
        s,
        "pass {passed_all}/{total} = {:.1}%",
        f64::from(passed_all) * 100.0 / total.max(1) as f64
    );
    for (title, m) in [("category", &by_cat), ("language", &by_lang)] {
        for (k, (p, t)) in m {
            let _ = writeln!(s, "  {title:<8} {k:<12} {p}/{t}");
        }
    }
    let _ = writeln!(
        s,
        "DANGEROUS runs: {dangerous} (critical cases: {dangerous_critical}; gate = 0)"
    );
    let _ = writeln!(
        s,
        "minor (title wording) {minor}  unstable cases {unstable}"
    );
    let _ = writeln!(
        s,
        "clarify: expected-and-correct cases {clar_ok}, guessed instead of asking {clar_guess}, wrongly asked {clar_wrongly_asked}"
    );
    let _ = writeln!(
        s,
        "latency ms p50 {}  p95 {}  tokens in/out {}/{}  peak model RAM {} MB",
        pct(&lat, 0.5),
        pct(&lat, 0.95),
        tokens.0,
        tokens.1,
        ram / 1_000_000
    );
    print!("{s}");
    Ok(())
}
