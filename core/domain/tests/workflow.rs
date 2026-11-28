//! Contract suite for the WF-03 reference state machine (report §5.3).
//!
//! Only the cases selected for this increment are covered: W-02, W-05 and W-06.
//! W-03/W-04 (retry and double click) remain an explicitly documented gap for WFE-01.

#![allow(clippy::unwrap_used, clippy::expect_used)]

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

fn proposal(circle: CircleId, policy_hash: &PolicyContextHash) -> ActionProposal {
    ActionProposal {
        id: ObjectId::new(),
        requester: PrincipalId::new(),
        target_circle: circle,
        action_type: ActionType::new("create_task").unwrap(),
        target_resource_refs: vec![],
        proposed_payload: Payload::new("Synthetic: call Anna tomorrow"),
        source_refs: vec![SourceRef {
            source: SourceId::new(),
            locator: None,
            captured_at: OffsetDateTime::UNIX_EPOCH,
        }],
        policy_context_hash: policy_hash.clone(),
        risk_class: RiskClass::Low,
        reversibility: Reversibility::Cancellable,
        idempotency_key: IdempotencyKey::new("idem-1").unwrap(),
        expires_at: at(10_000),
        approval_requirement: ApprovalRequirement::Required,
    }
}

fn context<'a>(
    now: i64,
    proposal_hash: &'a ProposalHash,
    policy_hash: &'a PolicyContextHash,
) -> TransitionContext<'a> {
    TransitionContext {
        now: at(now),
        proposal_hash,
        policy_context_hash: policy_hash,
    }
}

fn approval_for(workflow: &Workflow, hash: &ProposalHash, from: i64, until: i64) -> Approval {
    Approval {
        approver: PrincipalId::new(),
        at: at(from),
        expires_at: at(until),
        proposal_hash: hash.clone(),
        target_circle: workflow.proposal().target_circle,
    }
}

fn evaluate(workflow: &mut Workflow, ctx: &TransitionContext<'_>) {
    workflow
        .transition(&WorkflowEvent::ContextResolved, ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::PlanDrafted, ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::SchemaValidated, ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::PolicyEvaluated, ctx)
        .unwrap();
}

#[test]
fn w02_timeout_is_unknown_and_must_be_reconciled_before_any_retry() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    let mut workflow = Workflow::new(proposal(CircleId::new(), &policy));
    let ctx = context(100, &fingerprint, &policy);
    evaluate(&mut workflow, &ctx);

    let approval = approval_for(&workflow, &fingerprint, 150, 500);
    workflow
        .transition(
            &WorkflowEvent::Approved { approval },
            &context(150, &fingerprint, &policy),
        )
        .unwrap();
    let exec_ctx = context(250, &fingerprint, &policy);
    workflow
        .transition(&WorkflowEvent::ExecutionStarted, &exec_ctx)
        .unwrap();

    workflow
        .transition(&WorkflowEvent::TimedOut, &exec_ctx)
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::OutcomeUnknown);
    assert_ne!(workflow.state(), WorkflowState::Failed);
    assert_eq!(
        workflow.transition(&WorkflowEvent::ExecutionStarted, &exec_ctx),
        Err(TransitionError::InvalidTransition(
            "ExecutionStarted".to_owned(),
            WorkflowState::OutcomeUnknown
        ))
    );

    workflow
        .transition(&WorkflowEvent::Reconciled, &exec_ctx)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::Completed, &exec_ctx)
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::Completed);
}

#[test]
fn w05_changed_payload_target_or_window_invalidates_approval() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    let changed = ProposalHash::new("proposal-b").unwrap();
    let mut workflow = Workflow::new(proposal(CircleId::new(), &policy));
    let ctx = context(100, &fingerprint, &policy);
    evaluate(&mut workflow, &ctx);
    workflow
        .transition(&WorkflowEvent::ApprovalRequired, &ctx)
        .unwrap();

    let good = approval_for(&workflow, &fingerprint, 150, 500);
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::Approved {
                approval: good.clone(),
            },
            &context(200, &changed, &policy)
        ),
        Err(TransitionError::StaleApproval)
    );
    assert_eq!(workflow.state(), WorkflowState::ApprovalRequired);

    let wrong_target = Approval {
        target_circle: CircleId::new(),
        ..good.clone()
    };
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::Approved {
                approval: wrong_target,
            },
            &context(200, &fingerprint, &policy)
        ),
        Err(TransitionError::StaleApproval)
    );

    assert_eq!(
        workflow.transition(
            &WorkflowEvent::Approved {
                approval: good.clone(),
            },
            &context(600, &fingerprint, &policy)
        ),
        Err(TransitionError::ApprovalOutsideWindow)
    );

    workflow
        .transition(
            &WorkflowEvent::Approved { approval: good },
            &context(200, &fingerprint, &policy),
        )
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::Approved);
    assert!(workflow.approval().is_some());
}

#[test]
fn w06_revoked_policy_context_requires_re_evaluation_before_approval_and_execution() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let revoked = PolicyContextHash::new("policy-b").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    let mut workflow = Workflow::new(proposal(CircleId::new(), &policy));
    evaluate(&mut workflow, &context(100, &fingerprint, &policy));

    let approval = approval_for(&workflow, &fingerprint, 150, 500);
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::Approved {
                approval: approval.clone(),
            },
            &context(200, &fingerprint, &revoked)
        ),
        Err(TransitionError::PolicyContextChanged)
    );
    assert_eq!(workflow.state(), WorkflowState::PolicyEvaluated);

    workflow
        .transition(
            &WorkflowEvent::ApprovalRequired,
            &context(100, &fingerprint, &policy),
        )
        .unwrap();
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::Approved {
                approval: approval.clone(),
            },
            &context(200, &fingerprint, &revoked)
        ),
        Err(TransitionError::PolicyContextChanged)
    );

    workflow
        .transition(
            &WorkflowEvent::Approved { approval },
            &context(200, &fingerprint, &policy),
        )
        .unwrap();
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::ExecutionStarted,
            &context(250, &fingerprint, &revoked)
        ),
        Err(TransitionError::PolicyContextChanged)
    );
    assert_eq!(workflow.state(), WorkflowState::Approved);

    workflow
        .transition(
            &WorkflowEvent::ExecutionStarted,
            &context(250, &fingerprint, &policy),
        )
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::ExecutionStarted);
}

#[test]
fn workflow_rejects_unknown_fields_and_round_trips() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let workflow = Workflow::new(proposal(CircleId::new(), &policy));
    let value = serde_json::to_value(&workflow).unwrap();
    assert_eq!(
        serde_json::from_value::<Workflow>(value.clone()).unwrap(),
        workflow
    );

    let mut with_extra = value;
    with_extra["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Workflow>(with_extra).is_err());
}

#[test]
fn payload_and_errors_never_expose_content() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let mut input = proposal(CircleId::new(), &policy);
    input.proposed_payload = Payload::new("CANARY-9-9 hidden payload");
    let mut workflow = Workflow::new(input);

    assert!(!format!("{workflow:?}").contains("CANARY-"));
    assert!(!format!("{:?}", workflow.proposal().proposed_payload).contains("CANARY-"));

    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    let error = workflow
        .transition(
            &WorkflowEvent::Succeeded,
            &context(100, &fingerprint, &policy),
        )
        .unwrap_err();
    assert!(!error.to_string().contains("CANARY-"));
}

#[test]
fn execution_revalidates_expiry_proposal_binding_and_policy_context() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let revoked = PolicyContextHash::new("policy-b").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    let changed = ProposalHash::new("proposal-b").unwrap();
    let mut workflow = Workflow::new(proposal(CircleId::new(), &policy));
    evaluate(&mut workflow, &context(100, &fingerprint, &policy));

    let approval = approval_for(&workflow, &fingerprint, 150, 500);
    workflow
        .transition(
            &WorkflowEvent::Approved { approval },
            &context(200, &fingerprint, &policy),
        )
        .unwrap();

    assert_eq!(
        workflow.transition(
            &WorkflowEvent::ExecutionStarted,
            &context(600, &fingerprint, &policy)
        ),
        Err(TransitionError::ApprovalOutsideWindow)
    );
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::ExecutionStarted,
            &context(250, &changed, &policy)
        ),
        Err(TransitionError::StaleApproval)
    );
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::ExecutionStarted,
            &context(250, &fingerprint, &revoked)
        ),
        Err(TransitionError::PolicyContextChanged)
    );
    assert_eq!(workflow.state(), WorkflowState::Approved);

    workflow
        .transition(
            &WorkflowEvent::ExecutionStarted,
            &context(250, &fingerprint, &policy),
        )
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::ExecutionStarted);
}

#[test]
fn proposal_validity_window_closes_before_execution() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    let mut input = proposal(CircleId::new(), &policy);
    input.expires_at = at(300);
    let mut workflow = Workflow::new(input);
    evaluate(&mut workflow, &context(100, &fingerprint, &policy));

    let approval = approval_for(&workflow, &fingerprint, 150, 500);
    workflow
        .transition(
            &WorkflowEvent::Approved { approval },
            &context(200, &fingerprint, &policy),
        )
        .unwrap();
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::ExecutionStarted,
            &context(400, &fingerprint, &policy)
        ),
        Err(TransitionError::ProposalExpired)
    );
    assert_eq!(workflow.state(), WorkflowState::Approved);
}

#[test]
fn revoked_context_can_be_re_evaluated_and_denied() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let revoked = PolicyContextHash::new("policy-b").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();

    let mut workflow = Workflow::new(proposal(CircleId::new(), &policy));
    evaluate(&mut workflow, &context(100, &fingerprint, &policy));
    workflow
        .transition(
            &WorkflowEvent::ApprovalRequired,
            &context(100, &fingerprint, &policy),
        )
        .unwrap();
    let approval = approval_for(&workflow, &fingerprint, 150, 500);
    assert_eq!(
        workflow.transition(
            &WorkflowEvent::Approved { approval },
            &context(200, &fingerprint, &revoked)
        ),
        Err(TransitionError::PolicyContextChanged)
    );

    let re_evaluated = context(200, &fingerprint, &revoked);
    workflow
        .transition(&WorkflowEvent::PolicyEvaluated, &re_evaluated)
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::PolicyEvaluated);
    assert!(workflow.approval().is_none());
    workflow
        .transition(&WorkflowEvent::Denied, &re_evaluated)
        .unwrap();
    assert_eq!(workflow.state(), WorkflowState::Denied);

    let mut approved_workflow = Workflow::new(proposal(CircleId::new(), &policy));
    evaluate(&mut approved_workflow, &context(100, &fingerprint, &policy));
    let approval = approval_for(&approved_workflow, &fingerprint, 150, 500);
    approved_workflow
        .transition(
            &WorkflowEvent::Approved { approval },
            &context(200, &fingerprint, &policy),
        )
        .unwrap();
    approved_workflow
        .transition(&WorkflowEvent::PolicyEvaluated, &re_evaluated)
        .unwrap();
    assert!(approved_workflow.approval().is_none());
    approved_workflow
        .transition(&WorkflowEvent::Denied, &re_evaluated)
        .unwrap();
    assert_eq!(approved_workflow.state(), WorkflowState::Denied);
}

#[test]
fn first_evaluation_binds_the_proposal_context() {
    let policy = PolicyContextHash::new("policy-a").unwrap();
    let other = PolicyContextHash::new("policy-b").unwrap();
    let fingerprint = ProposalHash::new("proposal-a").unwrap();
    let mut workflow = Workflow::new(proposal(CircleId::new(), &policy));
    let wrong = context(100, &fingerprint, &other);
    workflow
        .transition(&WorkflowEvent::ContextResolved, &wrong)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::PlanDrafted, &wrong)
        .unwrap();
    workflow
        .transition(&WorkflowEvent::SchemaValidated, &wrong)
        .unwrap();
    assert_eq!(
        workflow.transition(&WorkflowEvent::PolicyEvaluated, &wrong),
        Err(TransitionError::PolicyContextMismatch)
    );
    assert_eq!(workflow.state(), WorkflowState::SchemaValidated);
}
