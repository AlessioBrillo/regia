//! Domain-level idempotency contract for W-03/W-04 (report §5.3, AR-08).
//!
//! A *compliant executor* is modelled here by an in-memory ledger keyed by `IdempotencyKey`.
//! These tests show that duplicate deliveries collapse when the caller keeps the key stable and
//! that the reference state machine refuses events repeated in a terminal state. They do **not**
//! verify an external side effect: that requires the simulator in WFE-01, which is still an
//! unexecuted spike (ADR-007 remains proposed).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use regia_domain::{
    ActionProposal, ActionType, Approval, ApprovalRequirement, CircleId, IdempotencyKey, ObjectId,
    Payload, PolicyContextHash, PrincipalId, ProposalHash, Reversibility, RiskClass, SourceId,
    SourceRef, Timestamp, TransitionContext, TransitionError, Workflow, WorkflowEvent,
    WorkflowState,
};
use time::OffsetDateTime;

fn at(secs: i64) -> Timestamp {
    Timestamp(OffsetDateTime::from_unix_timestamp(secs).unwrap())
}

fn proposal(key: &str, policy: &PolicyContextHash) -> ActionProposal {
    ActionProposal {
        id: ObjectId::new(),
        requester: PrincipalId::new(),
        target_circle: CircleId::new(),
        action_type: ActionType::new("create_task").unwrap(),
        target_resource_refs: vec![],
        proposed_payload: Payload::new("Synthetic: call Anna tomorrow"),
        source_refs: vec![SourceRef {
            source: SourceId::new(),
            locator: None,
            captured_at: OffsetDateTime::UNIX_EPOCH,
        }],
        policy_context_hash: policy.clone(),
        risk_class: RiskClass::Low,
        reversibility: Reversibility::Cancellable,
        idempotency_key: IdempotencyKey::new(key).unwrap(),
        expires_at: at(10_000),
        approval_requirement: ApprovalRequirement::Required,
    }
}

fn context<'a>(
    now: i64,
    proposal_hash: &'a ProposalHash,
    policy: &'a PolicyContextHash,
) -> TransitionContext<'a> {
    TransitionContext {
        now: at(now),
        proposal_hash,
        policy_context_hash: policy,
    }
}

struct Setup {
    workflow: Workflow,
    fingerprint: ProposalHash,
    policy: PolicyContextHash,
}

fn ready(key: &str) -> Setup {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    let mut workflow = Workflow::new(proposal(key, &policy));
    let ctx = context(100, &fingerprint, &policy);
    for event in [
        WorkflowEvent::ContextResolved,
        WorkflowEvent::PlanDrafted,
        WorkflowEvent::SchemaValidated,
        WorkflowEvent::PolicyEvaluated,
        WorkflowEvent::ApprovalRequired,
    ] {
        workflow.transition(&event, &ctx).unwrap();
    }
    let approval = Approval {
        approver: PrincipalId::new(),
        at: at(100),
        expires_at: at(500),
        proposal_hash: fingerprint.clone(),
        target_circle: workflow.proposal().target_circle,
    };
    workflow
        .transition(&WorkflowEvent::Approved { approval }, &ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::ExecutionStarted, &ctx)
        .unwrap();
    Setup {
        workflow,
        fingerprint,
        policy,
    }
}

/// An executor that performs at most one effect per idempotency key.
#[derive(Debug, Default)]
struct Ledger {
    seen: HashSet<IdempotencyKey>,
    effects: usize,
}

impl Ledger {
    fn execute(&mut self, key: &IdempotencyKey) -> usize {
        if self.seen.insert(key.clone()) {
            self.effects += 1;
        }
        self.effects
    }
}

#[test]
fn w03_retry_after_reconciliation_keeps_one_effect() {
    let Setup {
        mut workflow,
        fingerprint,
        policy,
    } = ready("idem-1");
    let key = workflow.proposal().idempotency_key.clone();
    let ctx = context(250, &fingerprint, &policy);
    let mut ledger = Ledger::default();

    assert_eq!(ledger.execute(&key), 1);
    workflow.transition(&WorkflowEvent::TimedOut, &ctx).unwrap();
    assert_eq!(workflow.state(), WorkflowState::OutcomeUnknown);
    workflow
        .transition(&WorkflowEvent::Reconciled, &ctx)
        .unwrap();
    // The retry carries the same key: the remote effect is not repeated.
    assert_eq!(ledger.execute(&key), 1);
    workflow
        .transition(&WorkflowEvent::Completed, &ctx)
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::Completed);
    assert_eq!(ledger.effects, 1);
}

#[test]
fn w04_duplicate_delivery_executes_once_and_the_state_is_unchanged() {
    let Setup {
        mut workflow,
        fingerprint,
        policy,
    } = ready("idem-2");
    let key = workflow.proposal().idempotency_key.clone();
    let ctx = context(250, &fingerprint, &policy);
    let mut ledger = Ledger::default();

    assert_eq!(ledger.execute(&key), 1);
    assert_eq!(ledger.execute(&key), 1);
    assert_eq!(
        workflow.transition(&WorkflowEvent::ExecutionStarted, &ctx),
        Err(TransitionError::InvalidTransition(
            "ExecutionStarted".to_owned(),
            WorkflowState::ExecutionStarted
        ))
    );
    assert_eq!(workflow.state(), WorkflowState::ExecutionStarted);
    assert_eq!(ledger.effects, 1);
}

#[test]
fn repeated_approval_and_completion_events_are_refused() {
    let Setup {
        mut workflow,
        fingerprint,
        policy,
    } = ready("idem-3");
    let ctx = context(250, &fingerprint, &policy);
    // Already in ExecutionStarted: a second approval or a second start is not an event.
    assert!(matches!(
        workflow.transition(
            &WorkflowEvent::Approved {
                approval: Approval {
                    approver: PrincipalId::new(),
                    at: at(100),
                    expires_at: at(500),
                    proposal_hash: fingerprint.clone(),
                    target_circle: workflow.proposal().target_circle,
                },
            },
            &ctx
        ),
        Err(TransitionError::InvalidTransition(
            _,
            WorkflowState::ExecutionStarted
        ))
    ));
    for event in [
        WorkflowEvent::Succeeded,
        WorkflowEvent::Reconciled,
        WorkflowEvent::Completed,
    ] {
        workflow.transition(&event, &ctx).unwrap();
    }
    assert!(matches!(
        workflow.transition(&WorkflowEvent::Completed, &ctx),
        Err(TransitionError::InvalidTransition(
            _,
            WorkflowState::Completed
        ))
    ));
}

#[test]
fn a_different_key_is_a_different_effect() {
    let mut ledger = Ledger::default();
    let first = IdempotencyKey::new("idem-4").unwrap();
    let second = IdempotencyKey::new("idem-5").unwrap();
    assert_eq!(ledger.execute(&first), 1);
    assert_eq!(ledger.execute(&first), 1);
    assert_eq!(ledger.execute(&second), 2);
    assert_eq!(ledger.effects, 2);
}
