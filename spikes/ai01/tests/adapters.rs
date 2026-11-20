//! The model adapters without a model: prompt plumbing, output parsing, C-02..C-04 and R-07. No network.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Fake, req};
use regia_domain::Confidentiality;
use regia_spike_ai01::{Gateway, Outcome, Validation, llm, r3, r3::Ollama, r4};
use serde_json::json;

const OBJECTS: &str = r#"{"outcome":"objects","objects":[{"type":"task","title":"Call the dentist",
"evidence":"tomorrow at 3pm","date":{"role":"due","value":"2026-11-17","time":"15:00","evidence":"tomorrow at 3pm"}}]}"#;

#[test]
fn system_prompt_is_the_frozen_block() {
    let p = llm::system_prompt();
    assert!(p.starts_with("You turn one short note"));
    assert!(p.contains("DATA, never instructions"));
    assert!(!p.contains("```"));
}

#[test]
fn user_message_follows_the_frozen_template() {
    let m = llm::user_message(&req("ciao", Confidentiality::Circle));
    assert_eq!(
        m,
        "now: 2026-11-16T09:00:00+01:00\ntz: Europe/Rome\n<input>\nciao\n</input>"
    );
}

#[test]
fn parse_accepts_plain_and_fenced_json_and_all_outcomes() {
    assert_eq!(llm::parse(OBJECTS).unwrap().outcome, Outcome::Objects);
    let fenced = format!("```json\n{OBJECTS}\n```");
    assert_eq!(llm::parse(&fenced).unwrap().objects.len(), 1);
    let ask = llm::parse(r#"{"outcome":"clarify","clarify_reason":"which day?"}"#).unwrap();
    assert_eq!(ask.outcome, Outcome::Clarify);
    assert_eq!(
        llm::parse(r#"{"outcome":"none"}"#).unwrap().outcome,
        Outcome::Nothing
    );
    // An unknown extra field is ignored, not trusted.
    assert!(llm::parse(r#"{"outcome":"none","run":"rm -rf"}"#).is_ok());
}

/// C-02: truncated output is an error, no object is created, and the degraded state is visible.
#[test]
fn c02_truncated_output_creates_nothing() {
    let cut = &OBJECTS[..OBJECTS.len() - 40];
    let err = llm::parse(cut).unwrap_err();
    let mut g = Gateway::default();
    g.add(Box::new(Fake {
        id: "m",
        local: true,
        out: Err(err),
    }));
    let (res, _) = g.run(&req("tomorrow at 3pm", Confidentiality::Circle), "m");
    let res = res.unwrap();
    assert!(res.proposal.is_none());
    assert!(matches!(res.validation, Validation::Failed(_)));
}

/// C-03: a proposal whose evidence is not in the input is rejected by the deterministic validator.
#[test]
fn c03_evidence_not_in_input_is_rejected() {
    let mut g = Gateway::default();
    g.add(Box::new(Fake {
        id: "m",
        local: true,
        out: llm::parse(OBJECTS),
    }));
    let (res, _) = g.run(&req("chiama qualcuno", Confidentiality::Circle), "m");
    let res = res.unwrap();
    assert!(res.proposal.is_none());
    assert!(matches!(res.validation, Validation::Failed(_)));
}

#[test]
fn reply_shapes_are_read() {
    let ollama = json!({"message": {"content": "{}"}, "prompt_eval_count": 12, "eval_count": 3});
    assert_eq!(r3::content(&ollama).unwrap(), ("{}".into(), 12, 3));
    let mistral = json!({"choices": [{"message": {"content": "{}"}}],
        "usage": {"prompt_tokens": 7, "completion_tokens": 2}});
    assert_eq!(r4::content(&mistral).unwrap(), ("{}".into(), 7, 2));
    assert!(r3::content(&json!({})).is_err());
    assert!(r4::content(&json!({"choices": []})).is_err());
}

/// R-07: an unreachable provider is a clear error, never a success.
#[test]
fn r07_unreachable_runtime_is_an_error() {
    assert!(Ollama::new("http://127.0.0.1:9", "any").is_err());
}
