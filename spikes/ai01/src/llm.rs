//! Plumbing shared by the model adapters (R3, R4): prompt `p1`, request building and output parsing.
//!
//! The system prompt is read from the frozen document itself, so the text that was frozen is the text that runs.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;

use crate::contract::{Proposal, TaskRequest};

const PROMPT_DOC: &str = include_str!("../../../docs/experiments/ai-01/prompt-v1.md");

/// Prompt version recorded with every run.
pub const PROMPT_VERSION: &str = "p1";

// ponytail: the golden set has one zone and `TaskRequest` has no `tz` field (frozen contract). Carry it with the
// request when a second zone exists.
const TZ: &str = "Europe/Rome";

static TOKENS_IN: AtomicU64 = AtomicU64::new(0);
static TOKENS_OUT: AtomicU64 = AtomicU64::new(0);

/// The first fenced block after the "System prompt" heading of a prompt document; line endings are normalised.
///
/// # Panics
/// The heading or the fence is missing: a run with an empty prompt must never be recorded as `p1`.
#[must_use]
pub fn extract_system_prompt(doc: &str) -> String {
    let doc = doc.replace("\r\n", "\n");
    let after = doc.split_once("## System prompt").map_or("", |(_, a)| a);
    let body = after.split_once("```text\n").map_or("", |(_, b)| b);
    let prompt = body.split_once("\n```").map_or("", |(p, _)| p);
    assert!(!prompt.trim().is_empty(), "system prompt block not found");
    prompt.to_owned()
}

/// The frozen system prompt `p1`.
#[must_use]
pub fn system_prompt() -> String {
    extract_system_prompt(PROMPT_DOC)
}

/// The user message, exactly as the frozen template: instant, zone, then the untrusted text.
#[must_use]
pub fn user_message(req: &TaskRequest) -> String {
    let now = req.now.format(&Rfc3339).unwrap_or_default();
    format!("now: {now}\ntz: {TZ}\n<input>\n{}\n</input>", req.input)
}

/// `[system, user]` chat messages, the shape both providers accept.
#[must_use]
pub fn messages(req: &TaskRequest) -> Value {
    json!([
        {"role": "system", "content": system_prompt()},
        {"role": "user", "content": user_message(req)},
    ])
}

/// An HTTP agent with a bounded wait: a slow or dead provider is an error, never a hang (R-07).
#[must_use]
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(180)))
        .build()
        .into()
}

/// Parses the model's text into a [`Proposal`]. A Markdown fence around the JSON is tolerated; nothing else is.
///
/// # Errors
/// The text is not a valid proposal (truncated, wrong shape): the caller discards it (C-02).
pub fn parse(raw: &str) -> Result<Proposal, String> {
    let t = raw.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .map_or(t, |s| s.trim_end().strip_suffix("```").unwrap_or(s))
        .trim();
    serde_json::from_str(t).map_err(|e| format!("unusable output: {e}"))
}

/// Adds the tokens of one call to the run counters.
pub fn add_tokens(input: u64, output: u64) {
    TOKENS_IN.fetch_add(input, Ordering::Relaxed);
    TOKENS_OUT.fetch_add(output, Ordering::Relaxed);
}

/// Returns and resets the tokens counted since the last call (in, out).
// ponytail: process-wide counters, valid because the runner is single-threaded.
#[must_use]
pub fn take_tokens() -> (u64, u64) {
    (
        TOKENS_IN.swap(0, Ordering::Relaxed),
        TOKENS_OUT.swap(0, Ordering::Relaxed),
    )
}
