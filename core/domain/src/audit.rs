//! Minimal audit record shape (report §4.12, AR-12, R-11).
//!
//! An audit record states *that* a decision was taken and by whom, under which policy version,
//! with which result and which sources it considered. It deliberately has no field for a prompt,
//! a payload, a name or any other content: minimisation is structural, not a convention that a
//! caller can forget. This is a data shape only; storage, retention, access control and the
//! telemetry backend remain open in [ADR-010](../../docs/adr/010-telemetry-and-audit.md).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    entity::Timestamp,
    id::{ObjectId, PrincipalId},
    provenance::SourceRef,
};

/// A policy version was blank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("policy version must not be blank")]
pub struct BlankPolicyVersion;

/// Version of the policy that produced a decision, supplied by the caller.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PolicyVersion(String);

impl PolicyVersion {
    /// Creates a version from caller-supplied text.
    ///
    /// # Errors
    ///
    /// [`BlankPolicyVersion`] if the text is blank.
    pub fn new(value: impl Into<String>) -> Result<Self, BlankPolicyVersion> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(BlankPolicyVersion)
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }

    /// The caller-supplied version text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PolicyVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PolicyVersion").field(&self.0).finish()
    }
}

/// The deterministic decision being recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AuditDecision {
    /// The action was allowed.
    Allow,
    /// The action was denied; an error is never mapped to an allow.
    Deny,
}

/// What happened after the decision, without describing the effect itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AuditResult {
    /// The action completed.
    Succeeded,
    /// The action failed.
    Failed,
    /// No execution was attempted.
    NotAttempted,
}

/// Content-free inputs of an audit record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    /// When the decision was taken.
    pub at: Timestamp,
    /// Who requested the action.
    pub actor: PrincipalId,
    /// Identity of the action or proposal.
    pub action: ObjectId,
    /// The deterministic decision.
    pub decision: AuditDecision,
    /// Policy version used for the decision.
    pub policy_version: PolicyVersion,
    /// Outcome after the decision.
    pub result: AuditResult,
    /// Sources considered, as references only.
    pub source_refs: Vec<SourceRef>,
}

/// An append-only, content-free audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    id: ObjectId,
    at: Timestamp,
    actor: PrincipalId,
    action: ObjectId,
    decision: AuditDecision,
    policy_version: PolicyVersion,
    result: AuditResult,
    source_refs: Vec<SourceRef>,
}

impl AuditEvent {
    /// Records an event. There is no update method: records are append-only.
    ///
    /// The record identity is the action identity: one decision produces one audit event.
    #[must_use]
    pub fn new(record: AuditRecord) -> Self {
        Self {
            id: record.action,
            at: record.at,
            actor: record.actor,
            action: record.action,
            decision: record.decision,
            policy_version: record.policy_version,
            result: record.result,
            source_refs: record.source_refs,
        }
    }

    /// Record identity.
    #[must_use]
    pub const fn id(&self) -> &ObjectId {
        &self.id
    }

    /// When the decision was taken.
    #[must_use]
    pub const fn at(&self) -> Timestamp {
        self.at
    }

    /// Who requested the action.
    #[must_use]
    pub const fn actor(&self) -> &PrincipalId {
        &self.actor
    }

    /// Identity of the action or proposal.
    #[must_use]
    pub const fn action(&self) -> &ObjectId {
        &self.action
    }

    /// The deterministic decision.
    #[must_use]
    pub const fn decision(&self) -> AuditDecision {
        self.decision
    }

    /// Policy version used for the decision.
    #[must_use]
    pub const fn policy_version(&self) -> &PolicyVersion {
        &self.policy_version
    }

    /// Outcome after the decision.
    #[must_use]
    pub const fn result(&self) -> AuditResult {
        self.result
    }

    /// Sources considered, as references only.
    #[must_use]
    pub fn source_refs(&self) -> &[SourceRef] {
        &self.source_refs
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;

    fn record() -> AuditRecord {
        AuditRecord {
            at: Timestamp(OffsetDateTime::UNIX_EPOCH),
            actor: PrincipalId::new(),
            action: ObjectId::new(),
            decision: AuditDecision::Deny,
            policy_version: PolicyVersion::new("cedar-v1").unwrap(),
            result: AuditResult::NotAttempted,
            source_refs: vec![],
        }
    }

    #[test]
    fn record_has_no_content_fields() {
        let event = AuditEvent::new(record());
        let value = serde_json::to_value(&event).unwrap();
        for content_field in ["payload", "prompt", "text", "content", "name"] {
            assert!(value.get(content_field).is_none(), "{content_field}");
        }
        assert!(!serde_json::to_string(&event).unwrap().contains("CANARY-"));
    }

    #[test]
    fn unknown_fields_with_content_are_rejected() {
        let mut value = serde_json::to_value(AuditEvent::new(record())).unwrap();
        value["payload"] = serde_json::json!("CANARY-9-9 hidden payload");
        assert!(serde_json::from_value::<AuditEvent>(value).is_err());
    }

    #[test]
    fn blank_policy_version_is_unrepresentable() {
        assert!(PolicyVersion::new("  ").is_err());
        assert_eq!(
            PolicyVersion::new(" cedar-v1 ").unwrap().as_str(),
            "cedar-v1"
        );
    }

    #[test]
    fn json_roundtrip() {
        let event = AuditEvent::new(record());
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(event, serde_json::from_str::<AuditEvent>(&json).unwrap());
    }
}
