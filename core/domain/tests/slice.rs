//! Contract suite for the governed action slice: capture → proposal → approval → audit.
//!
//! This checks the wiring of types the report already fixes. It is not a policy engine, a
//! workflow runtime or a storage decision (ADR-006/007 and ADR-010 remain open).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use regia_domain::{
    Approval, AuditDecision, AuditEvent, AuditRecord, AuditResult, CircleId, PolicyContextHash,
    PolicyVersion, PrincipalId, ProposalHash, Reversibility, RiskClass, Timestamp,
    TransitionContext, TransitionError, Workflow, WorkflowEvent, WorkflowState,
    synthetic::{ActionDraft, capture_text_in_circle, propose_action_for_task},
};
use time::OffsetDateTime;

fn at(secs: i64) -> Timestamp {
    Timestamp(OffsetDateTime::from_unix_timestamp(secs).unwrap())
}

fn setup() -> (
    Workflow,
    PolicyContextHash,
    ProposalHash,
    PrincipalId,
    CircleId,
) {
    let task =
        capture_text_in_circle("Synthetic: call Anna tomorrow", CircleId::new(), at(0)).unwrap();
    let requester = PrincipalId::new();
    let proposal = propose_action_for_task(
        &task,
        requester,
        ActionDraft {
            action_type: "create_task",
            policy_context_hash: "policy-a",
            idempotency_key: "idem-1",
            risk_class: RiskClass::Low,
            reversibility: Reversibility::Cancellable,
            expires_at: at(100),
        },
    )
    .unwrap();
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    (
        Workflow::new(proposal),
        policy,
        fingerprint,
        requester,
        task.header.circle,
    )
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

fn audit(
    workflow: &Workflow,
    actor: PrincipalId,
    decision: AuditDecision,
    result: AuditResult,
) -> AuditEvent {
    AuditEvent::new(AuditRecord {
        at: at(10),
        actor,
        action: *workflow.id(),
        decision,
        policy_version: PolicyVersion::new("demo-policy-v1").unwrap(),
        result,
        source_refs: workflow.proposal().source_refs.clone(),
    })
}

#[test]
fn approved_slice_records_an_allow_without_recording_content() {
    let (mut workflow, policy, fingerprint, requester, circle) = setup();
    let ctx = TransitionContext {
        now: at(10),
        proposal_hash: &fingerprint,
        policy_context_hash: &policy,
    };
    evaluate(&mut workflow, &ctx);
    workflow
        .transition(&WorkflowEvent::ApprovalRequired, &ctx)
        .unwrap();
    let approval = Approval {
        approver: PrincipalId::new(),
        at: at(10),
        expires_at: at(50),
        proposal_hash: fingerprint.clone(),
        target_circle: circle,
    };
    workflow
        .transition(&WorkflowEvent::Approved { approval }, &ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::ExecutionStarted, &ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::Succeeded, &ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::Reconciled, &ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::Completed, &ctx)
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::Completed);

    let event = audit(
        &workflow,
        requester,
        AuditDecision::Allow,
        AuditResult::Succeeded,
    );
    let json = serde_json::to_string(&event).unwrap();
    assert!(!json.contains("Anna"));
    assert!(!json.contains("CANARY-"));
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    for content_field in ["payload", "prompt", "text", "content", "name"] {
        assert!(value.get(content_field).is_none(), "{content_field}");
    }
    assert_eq!(event.decision(), AuditDecision::Allow);
    assert_eq!(event.result(), AuditResult::Succeeded);
    assert_eq!(event.source_refs().len(), 1);
}

#[test]
fn denied_slice_never_executes_and_records_a_deny() {
    let (mut workflow, policy, fingerprint, requester, _circle) = setup();
    let ctx = TransitionContext {
        now: at(10),
        proposal_hash: &fingerprint,
        policy_context_hash: &policy,
    };
    evaluate(&mut workflow, &ctx);
    workflow.transition(&WorkflowEvent::Denied, &ctx).unwrap();
    assert_eq!(workflow.state(), WorkflowState::Denied);
    assert!(workflow.approval().is_none());

    let event = audit(
        &workflow,
        requester,
        AuditDecision::Deny,
        AuditResult::NotAttempted,
    );
    assert_eq!(event.decision(), AuditDecision::Deny);
    assert_eq!(event.result(), AuditResult::NotAttempted);
    assert!(!serde_json::to_string(&event).unwrap().contains("CANARY-"));
}

#[test]
fn approval_from_another_circle_is_refused_leaving_the_state_unchanged() {
    let (mut workflow, policy, fingerprint, _requester, _circle) = setup();
    let ctx = TransitionContext {
        now: at(10),
        proposal_hash: &fingerprint,
        policy_context_hash: &policy,
    };
    evaluate(&mut workflow, &ctx);
    workflow
        .transition(&WorkflowEvent::ApprovalRequired, &ctx)
        .unwrap();

    let wrong_circle = Approval {
        target_circle: CircleId::new(),
        approver: PrincipalId::new(),
        at: at(10),
        expires_at: at(50),
        proposal_hash: fingerprint.clone(),
    };
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::Approved {
                approval: wrong_circle,
            },
            &ctx
        ),
        Err(TransitionError::StaleApproval)
    );
    assert_eq!(workflow.state(), WorkflowState::ApprovalRequired);
}
