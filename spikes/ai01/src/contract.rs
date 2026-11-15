//! `TaskRequest` / `TaskResult` v0 and the adapter trait (ADR-008 candidate; a fixture-level proposal).

use regia_domain::{Confidentiality, DateRole};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Contract version carried in every audit record.
pub const CONTRACT_VERSION: &str = "0";

/// What the caller wants done. One family in increment 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Intent parsing and extraction from short text (WF-01).
    IntentExtraction,
}

/// A request. The router sees only the metadata fields; `input` is read by the chosen adapter alone.
#[derive(Debug, Clone)]
pub struct TaskRequest {
    /// Task family.
    pub task_type: TaskType,
    /// The user's text (untrusted data, never instructions).
    pub input: String,
    /// Confidentiality label of the input.
    pub label: Confidentiality,
    /// Reference instant for relative dates.
    pub now: OffsetDateTime,
    /// The task must work with no network.
    pub offline_required: bool,
    /// The user explicitly agreed to send this task to a cloud runtime.
    pub cloud_consent: bool,
}

/// What the extraction decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Propose the listed objects.
    Objects,
    /// Ask the user; create nothing.
    Clarify,
    /// Not a capture; create nothing.
    #[serde(rename = "none")]
    Nothing,
}

/// Proposed object type. `Note` and `Document` dates are spike output only, not `regia-domain` types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjKind {
    /// Something to do.
    Task,
    /// Something at a time.
    Event,
    /// Free text.
    Note,
    /// A document with dated roles.
    Document,
}

/// A date with its meaning. `time` is `HH:MM`, `value` is `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dated {
    /// Meaning of the date.
    pub role: DateRole,
    /// ISO date.
    pub value: String,
    /// Optional time of day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// Verbatim text the date came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// One proposed object. Always a proposal (`Assertion::Proposed`), never a confirmed fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obj {
    /// Object type.
    #[serde(rename = "type")]
    pub kind: ObjKind,
    /// Free wording.
    pub title: String,
    /// Verbatim substring of the input that supports the object (AR-06).
    pub evidence: String,
    /// Single date (tasks, events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<Dated>,
    /// Dated roles (documents).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dates: Vec<Dated>,
    /// Decimal amount, `1200.00`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_eur: Option<String>,
    /// RFC 3339 reminder instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remind_at: Option<String>,
}

/// An adapter's whole output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    /// What to do with it.
    pub outcome: Outcome,
    /// Objects, empty unless `outcome` is `Objects`.
    #[serde(default)]
    pub objects: Vec<Obj>,
    /// Why the user is asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarify_reason: Option<String>,
}

/// Result of the deterministic validation that follows every adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validation {
    /// Schema, dates and evidence all check out.
    Valid,
    /// Rejected; no object is created.
    Failed(String),
}

/// What the gateway returns to the caller.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// The proposal; `None` when validation failed or the adapter errored.
    pub proposal: Option<Proposal>,
    /// Adapter that produced it.
    pub runtime: String,
    /// Outcome of validation.
    pub validation: Validation,
    /// Wall time of the adapter call.
    pub latency_ms: u64,
    /// Set when the route differs from the preferred runtime.
    pub fallback_reason: Option<String>,
}

/// A replaceable runtime. One thin adapter per runtime (ADR-008 option A).
pub trait Adapter {
    /// Stable id, e.g. `r0`.
    fn id(&self) -> &str;
    /// Exact model or rules version.
    fn version(&self) -> &str;
    /// Runs on this device with no network.
    fn local(&self) -> bool;
    /// Extract a proposal. Errors are plain strings and never fall back to another runtime.
    ///
    /// # Errors
    /// The runtime failed or returned unusable output.
    fn extract(&self, req: &TaskRequest) -> Result<Proposal, String>;
}

/// Expectations of a golden case beyond the proposal itself.
#[derive(Debug, Clone, Deserialize)]
pub struct Expect {
    /// Expected outcome.
    pub outcome: Outcome,
    /// Expected objects.
    #[serde(default)]
    pub objects: Vec<Obj>,
    /// A role that must not appear on any produced date.
    #[serde(default)]
    pub must_not_have_role: Option<DateRole>,
    /// The input carries an injected instruction.
    #[serde(default)]
    pub injection: bool,
    /// The case must never reach a cloud runtime.
    #[serde(default)]
    pub cloud_forbidden: bool,
}

/// One line of `golden/intent-v1.jsonl`.
#[derive(Debug, Clone, Deserialize)]
pub struct Golden {
    /// `G-nnn`.
    pub id: String,
    /// `easy`, `ambiguous`, `adversarial`, `ood` or `canary`.
    pub category: String,
    /// `it` or `en`.
    pub lang: String,
    /// Confidentiality label.
    pub label: Confidentiality,
    /// Counts toward the zero-dangerous-error gate.
    pub critical: bool,
    /// Reference instant, RFC 3339.
    pub now: String,
    /// The text.
    pub input: String,
    /// What should come out.
    pub expect: Expect,
}

impl Golden {
    /// Builds the request for this case.
    ///
    /// # Errors
    /// `now` is not RFC 3339.
    pub fn request(&self) -> Result<TaskRequest, String> {
        let now = OffsetDateTime::parse(&self.now, &time::format_description::well_known::Rfc3339)
            .map_err(|e| e.to_string())?;
        Ok(TaskRequest {
            task_type: TaskType::IntentExtraction,
            input: self.input.clone(),
            label: self.label,
            now,
            offline_required: false,
            cloud_consent: false,
        })
    }

    /// Loads the golden file.
    ///
    /// # Errors
    /// File unreadable or a line is not a valid case.
    pub fn load(path: &std::path::Path) -> Result<Vec<Self>, String> {
        std::fs::read_to_string(path)
            .map_err(|e| e.to_string())?
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| e.to_string()))
            .collect()
    }
}
