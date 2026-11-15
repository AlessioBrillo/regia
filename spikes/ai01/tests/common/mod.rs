//! Shared helpers: a scriptable fake adapter and the golden loader.

#![allow(dead_code)]

use std::path::PathBuf;

use regia_domain::Confidentiality;
use regia_spike_ai01::{Adapter, Golden, Proposal, TaskRequest, contract::TaskType};
use time::OffsetDateTime;

pub struct Fake {
    pub id: &'static str,
    pub local: bool,
    pub out: Result<Proposal, String>,
}

impl Adapter for Fake {
    fn id(&self) -> &str {
        self.id
    }
    fn version(&self) -> &'static str {
        "fake-1"
    }
    fn local(&self) -> bool {
        self.local
    }
    fn extract(&self, _: &TaskRequest) -> Result<Proposal, String> {
        self.out.clone()
    }
}

pub fn golden() -> Vec<Golden> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/experiments/ai-01/golden/intent-v1.jsonl");
    Golden::load(&p).unwrap()
}

pub fn req(input: &str, label: Confidentiality) -> TaskRequest {
    TaskRequest {
        task_type: TaskType::IntentExtraction,
        input: input.into(),
        label,
        now: OffsetDateTime::parse(
            "2026-11-16T09:00:00+01:00",
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap(),
        offline_required: false,
        cloud_consent: true,
    }
}

pub fn nothing() -> Proposal {
    Proposal {
        outcome: regia_spike_ai01::Outcome::Nothing,
        objects: vec![],
        clarify_reason: None,
    }
}
