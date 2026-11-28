//! Demo-only deterministic fingerprints for the governed-action slice (AR-06, AR-12).
//!
//! [`crate::workflow`] deliberately treats `ProposalHash` and `PolicyContextHash` as opaque,
//! caller-supplied strings: this crate never derives them and never implements a cryptographic
//! primitive (report §4.5). The `act` demo, however, must not exercise authorisation checks with
//! two constant strings — that would make approval binding vacuous. These helpers fill that gap
//! for the demo only, using the maintained `sha2` library behind the `synthetic` feature.
//!
//! They are **not** a product contract: no ADR is accepted by their existence, nothing is stored
//! by them, and they decide nothing about policy, storage or workflow runtime. The domain
//! separation prefixes keep a proposal fingerprint and a policy-context fingerprint distinct
//! even when their JSON inputs happen to be equal.

use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{ActionProposal, BlankValue, CircleId, PolicyContextHash, PrincipalId, ProposalHash};

/// Prefix hashed before a serialised proposal.
const PROPOSAL_DOMAIN: &[u8] = b"regia-demo-proposal-v1";
/// Prefix hashed before a serialised policy context.
const CONTEXT_DOMAIN: &[u8] = b"regia-demo-policy-context-v1";

/// A fingerprint input could not be serialised, or was blank.
#[derive(Debug, thiserror::Error)]
pub enum FingerprintError {
    /// The input could not be serialised to JSON.
    #[error("cannot serialise fingerprint input: {0}")]
    Serialise(#[from] serde_json::Error),
    /// The derived value was blank; this cannot happen for a hex digest.
    #[error(transparent)]
    Blank(#[from] BlankValue),
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn digest(domain: &[u8], input: &impl Serialize) -> Result<String, FingerprintError> {
    // Struct field order is the canonicalisation here (serde_json emits fields in declaration
    // order). This is not a general JSON canonical form and is not claimed to be one.
    let bytes = serde_json::to_vec(input)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    Ok(hex(&hasher.finalize()))
}

/// Derives the demo fingerprint of a whole proposal.
///
/// A changed payload, target, source, idempotency key or validity window changes the result, so
/// an approval bound to one proposal cannot be reused for another.
///
/// # Errors
///
/// [`FingerprintError`] if the proposal cannot be serialised or the digest is blank.
pub fn proposal_fingerprint(proposal: &ActionProposal) -> Result<ProposalHash, FingerprintError> {
    Ok(ProposalHash::new(digest(PROPOSAL_DOMAIN, proposal)?)?)
}

/// Derives the demo fingerprint of the context a policy decision was taken against.
///
/// The tuple is `(declared decision, policy version, circle, requester)`: the inputs the demo
/// actually has. `decision` is supplied by the caller; the model never decides authorisation.
///
/// # Errors
///
/// [`FingerprintError`] if the context cannot be serialised or the digest is blank.
pub fn policy_context_fingerprint(
    decision: &str,
    policy_version: &str,
    circle: CircleId,
    requester: PrincipalId,
) -> Result<PolicyContextHash, FingerprintError> {
    let input = (decision, policy_version, circle, requester);
    Ok(PolicyContextHash::new(digest(CONTEXT_DOMAIN, &input)?)?)
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;
    use crate::{
        Payload, Reversibility, RiskClass, Timestamp,
        synthetic::{ActionDraft, capture_text_in_circle, propose_action_for_task},
    };

    fn proposal(text: &str, circle: CircleId) -> ActionProposal {
        let at = Timestamp(OffsetDateTime::UNIX_EPOCH);
        let task = capture_text_in_circle(text, circle, at).unwrap();
        propose_action_for_task(
            &task,
            PrincipalId::new(),
            ActionDraft {
                action_type: "create_task",
                policy_context_hash: "policy-v1",
                idempotency_key: "idem-v1",
                risk_class: RiskClass::Low,
                reversibility: Reversibility::Cancellable,
                expires_at: Timestamp(OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(10)),
            },
        )
        .unwrap()
    }

    fn hex_digest(domain: &[u8], input: &impl Serialize) -> String {
        digest(domain, input).unwrap()
    }

    #[test]
    fn proposal_fingerprint_is_deterministic_and_binding() {
        let p = proposal("Synthetic: call Anna tomorrow", CircleId::new());
        let base = proposal_fingerprint(&p).unwrap();
        assert_eq!(base, proposal_fingerprint(&p).unwrap());
        assert_eq!(base.as_str().len(), 64);
        assert!(base.as_str().bytes().all(|b| b.is_ascii_hexdigit()));

        let changed_payload = ActionProposal {
            proposed_payload: Payload::new("Synthetic: call Anna today"),
            ..p.clone()
        };
        let changed_circle = ActionProposal {
            target_circle: CircleId::new(),
            ..p.clone()
        };
        let changed_key = ActionProposal {
            idempotency_key: crate::IdempotencyKey::new("idem-v2").unwrap(),
            ..p
        };
        assert_ne!(base, proposal_fingerprint(&changed_payload).unwrap());
        assert_ne!(base, proposal_fingerprint(&changed_circle).unwrap());
        assert_ne!(base, proposal_fingerprint(&changed_key).unwrap());
    }

    #[test]
    fn context_fingerprint_is_deterministic_and_binding() {
        let circle = CircleId::new();
        let requester = PrincipalId::new();
        let base = policy_context_fingerprint("allow", "policy-v1", circle, requester).unwrap();
        assert_eq!(
            base,
            policy_context_fingerprint("allow", "policy-v1", circle, requester).unwrap()
        );
        assert_eq!(base.as_str().len(), 64);
        assert_ne!(
            base,
            policy_context_fingerprint("deny", "policy-v1", circle, requester).unwrap()
        );
        assert_ne!(
            base,
            policy_context_fingerprint("allow", "policy-v2", circle, requester).unwrap()
        );
        assert_ne!(
            base,
            policy_context_fingerprint("allow", "policy-v1", CircleId::new(), requester).unwrap()
        );
        assert_ne!(
            base,
            policy_context_fingerprint("allow", "policy-v1", circle, PrincipalId::new()).unwrap()
        );
    }

    #[test]
    fn domains_are_separated() {
        assert_ne!(
            hex_digest(PROPOSAL_DOMAIN, &"same input"),
            hex_digest(CONTEXT_DOMAIN, &"same input")
        );
    }
}
