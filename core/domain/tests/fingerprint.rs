//! Integration of the demo fingerprint derivation with the WF-03 approval checks.
//!
//! This covers the wiring the `act` example relies on: hashes derived from real bytes feed the
//! state machine, and a change to those bytes invalidates an approval. The state-machine rules
//! themselves are already covered by `tests/workflow.rs`; nothing here is a policy engine, a
//! workflow runtime or a product fingerprint contract (ADR-007/008 remain open).

#![cfg(feature = "synthetic")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use regia_domain::{
    ActionProposal, Approval, CircleId, Payload, PolicyContextHash, PrincipalId, ProposalHash,
    Reversibility, RiskClass, Timestamp, TransitionContext, TransitionError, Workflow,
    WorkflowEvent, WorkflowState, policy_context_fingerprint, proposal_fingerprint,
    synthetic::{ActionDraft, capture_text_in_circle, propose_action_for_task},
};
use time::OffsetDateTime;

fn at(secs: i64) -> Timestamp {
    Timestamp(OffsetDateTime::from_unix_timestamp(secs).unwrap())
}

fn proposal(text: &str, policy: &PolicyContextHash, circle: CircleId) -> ActionProposal {
    let task = capture_text_in_circle(text, circle, at(0)).unwrap();
    propose_action_for_task(
        &task,
        PrincipalId::new(),
        ActionDraft {
            action_type: "create_task",
            policy_context_hash: policy.as_str(),
            idempotency_key: "idem-v1",
            risk_class: RiskClass::Low,
            reversibility: Reversibility::Cancellable,
            expires_at: at(100),
        },
    )
    .unwrap()
}

fn context<'a>(
    now: i64,
    proposal: &'a ProposalHash,
    policy: &'a PolicyContextHash,
) -> TransitionContext<'a> {
    TransitionContext {
        now: at(now),
        proposal_hash: proposal,
        policy_context_hash: policy,
    }
}

fn evaluate(workflow: &mut Workflow, ctx: &TransitionContext<'_>) {
    for event in [
        WorkflowEvent::ContextResolved,
        WorkflowEvent::PlanDrafted,
        WorkflowEvent::SchemaValidated,
        WorkflowEvent::PolicyEvaluated,
    ] {
        workflow.transition(&event, ctx).unwrap();
    }
}

fn approval(hash: &ProposalHash, circle: CircleId) -> Approval {
    Approval {
        approver: PrincipalId::new(),
        at: at(10),
        expires_at: at(50),
        proposal_hash: hash.clone(),
        target_circle: circle,
    }
}

#[test]
fn derived_hashes_drive_the_whole_approval_chain() {
    let circle = CircleId::new();
    let requester = PrincipalId::new();
    let policy = policy_context_fingerprint("allow", "policy-v1", circle, requester).unwrap();
    let proposal = proposal("Synthetic: call Anna tomorrow", &policy, circle);
    let fingerprint = proposal_fingerprint(&proposal).unwrap();
    let ctx = context(10, &fingerprint, &policy);

    let mut workflow = Workflow::new(proposal);
    evaluate(&mut workflow, &ctx);
    assert_eq!(workflow.state(), WorkflowState::PolicyEvaluated);
    workflow
        .transition(&WorkflowEvent::ApprovalRequired, &ctx)
        .unwrap();
    workflow
        .transition(
            &WorkflowEvent::Approved {
                approval: approval(&fingerprint, circle),
            },
            &ctx,
        )
        .unwrap();
    for event in [
        WorkflowEvent::ExecutionStarted,
        WorkflowEvent::Succeeded,
        WorkflowEvent::Reconciled,
        WorkflowEvent::Completed,
    ] {
        workflow.transition(&event, &ctx).unwrap();
    }
    assert_eq!(workflow.state(), WorkflowState::Completed);
}

#[test]
fn a_changed_payload_invalidates_the_derived_approval() {
    let circle = CircleId::new();
    let policy =
        policy_context_fingerprint("allow", "policy-v1", circle, PrincipalId::new()).unwrap();
    let first = proposal("Synthetic: call Anna tomorrow", &policy, circle);
    let first_hash = proposal_fingerprint(&first).unwrap();
    let changed = ActionProposal {
        proposed_payload: Payload::new("Synthetic: call Anna today"),
        ..first.clone()
    };
    let changed_hash = proposal_fingerprint(&changed).unwrap();
    assert_ne!(first_hash, changed_hash);

    let mut workflow = Workflow::new(first);
    let ctx = context(10, &first_hash, &policy);
    evaluate(&mut workflow, &ctx);
    workflow
        .transition(&WorkflowEvent::ApprovalRequired, &ctx)
        .unwrap();
    workflow
        .transition(
            &WorkflowEvent::Approved {
                approval: approval(&first_hash, circle),
            },
            &ctx,
        )
        .unwrap();

    // The same approval cannot authorise a proposal whose real content changed.
    let changed_ctx = context(20, &changed_hash, &policy);
    assert_eq!(
        workflow.transition(&WorkflowEvent::ExecutionStarted, &changed_ctx),
        Err(TransitionError::StaleApproval)
    );
    assert_eq!(workflow.state(), WorkflowState::Approved);
}

#[test]
fn a_changed_policy_context_invalidates_the_derived_binding() {
    let circle = CircleId::new();
    let requester = PrincipalId::new();
    let allow = policy_context_fingerprint("allow", "policy-v1", circle, requester).unwrap();
    let deny = policy_context_fingerprint("deny", "policy-v1", circle, requester).unwrap();
    assert_ne!(allow, deny);

    let proposal = proposal("Synthetic: call Anna tomorrow", &allow, circle);
    let fingerprint = proposal_fingerprint(&proposal).unwrap();
    let mut workflow = Workflow::new(proposal);
    evaluate(&mut workflow, &context(10, &fingerprint, &allow));

    assert_eq!(
        workflow.transition(
            &WorkflowEvent::Approved {
                approval: approval(&fingerprint, circle),
            },
            &context(20, &fingerprint, &deny),
        ),
        Err(TransitionError::PolicyContextChanged)
    );
    assert_eq!(workflow.state(), WorkflowState::PolicyEvaluated);
}
