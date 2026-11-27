//! WF-03 action proposals and state machine (report §5.3, AR-07, AR-08, R-06).
//!
//! This module is a *reference specification* of the state machine the technical report
//! already fixes. It is not a workflow runtime: it does not persist anything, does not call a
//! policy engine, does not compute hashes and does not choose between a state machine with an
//! outbox and durable execution ([ADR-007](../../docs/adr/007-workflow-runtime.md), still
//! proposed). The model may produce a proposal; only deterministic code outside the model may
//! authorise it.
//!
//! Hashes are supplied by the caller as opaque, non-blank strings. Re-evaluating a changed
//! payload, target or permission context must produce a different value; this crate never
//! derives one and never implements a cryptographic primitive (§4.5).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    entity::Timestamp,
    id::{CircleId, ObjectId, PrincipalId},
    provenance::SourceRef,
};

/// A caller-supplied value was blank.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0} must not be blank")]
pub struct BlankValue(&'static str);

fn non_blank(value: impl Into<String>, what: &'static str) -> Result<String, BlankValue> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(BlankValue(what))
    } else {
        Ok(trimmed.to_owned())
    }
}

macro_rules! opaque_string {
    ($name:ident, $what:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a value from caller-supplied text.
            ///
            /// # Errors
            ///
            /// [`BlankValue`] if the text is blank.
            pub fn new(value: impl Into<String>) -> Result<Self, BlankValue> {
                Ok(Self(non_blank(value, $what)?))
            }

            /// The caller-supplied text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }
    };
}

opaque_string!(
    ActionType,
    "action type",
    "Declared action class, for example `create_task`. Not a permission."
);
opaque_string!(
    ProposalHash,
    "proposal hash",
    "Caller-supplied fingerprint of an `ActionProposal`; a changed payload or target must change it."
);
opaque_string!(
    PolicyContextHash,
    "policy context hash",
    "Caller-supplied fingerprint of the policy context evaluated for an action."
);
opaque_string!(
    IdempotencyKey,
    "idempotency key",
    "Stable key an executor must use so that a retry cannot duplicate an external effect."
);

/// Payload text, treated as opaque and never rendered in logs or diagnostics.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Payload(String);

impl Payload {
    /// Wraps caller-supplied, already-minimised payload text.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Debug for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Payload(<redacted>)")
    }
}

/// Declared risk class of an action. Provisional vocabulary, not a policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RiskClass {
    /// Routine, reversible local change.
    Low,
    /// Action with a visible effect on other people or on shared data.
    Elevated,
    /// Action whose mistaken execution is a critical incident.
    Critical,
}

/// Whether an action can be undone, compensated for, or not at all (report §5.3, W-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Reversibility {
    /// Can be cancelled before it takes effect.
    Cancellable,
    /// Cannot be cancelled, but a compensating action exists.
    Compensable,
    /// Neither cancellation nor compensation exists.
    Irreversible,
}

/// Whether a person must approve before execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ApprovalRequirement {
    /// No human approval is declared for this action class.
    NotRequired,
    /// A human approval is required before execution.
    Required,
}

/// What the model proposes. The backend, not the model, decides whether it may run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionProposal {
    /// Stable action identity.
    pub id: ObjectId,
    /// Who requested the action.
    pub requester: PrincipalId,
    /// Circle the action is scoped to.
    pub target_circle: CircleId,
    /// Declared action class.
    pub action_type: ActionType,
    /// Objects the action touches, when there are any.
    pub target_resource_refs: Vec<ObjectId>,
    /// Opaque payload; never copied into telemetry or an audit record.
    pub proposed_payload: Payload,
    /// Evidence behind the proposal (AR-06).
    pub source_refs: Vec<SourceRef>,
    /// Fingerprint of the policy context this proposal was built against.
    pub policy_context_hash: PolicyContextHash,
    /// Declared risk class.
    pub risk_class: RiskClass,
    /// Declared reversibility.
    pub reversibility: Reversibility,
    /// Key an executor must use to make retries safe.
    pub idempotency_key: IdempotencyKey,
    /// Latest instant at which an approval for this proposal is valid.
    pub expires_at: Timestamp,
    /// Declared approval requirement.
    pub approval_requirement: ApprovalRequirement,
}

/// A person's approval, bound to one proposal and one target circle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    /// Who approved.
    pub approver: PrincipalId,
    /// When the approval was given.
    pub at: Timestamp,
    /// When the approval stops being valid.
    pub expires_at: Timestamp,
    /// Fingerprint of the proposal that was approved.
    pub proposal_hash: ProposalHash,
    /// Circle that was approved, so a changed target cannot reuse this approval.
    pub target_circle: CircleId,
}

/// States of the state machine in report §5.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowState {
    /// Request received, nothing resolved yet.
    Requested,
    /// Context resolved.
    ContextResolved,
    /// Plan drafted.
    PlanDrafted,
    /// Draft validated against the output schema.
    SchemaValidated,
    /// Policy evaluated by deterministic code outside the model.
    PolicyEvaluated,
    /// Policy denied the action; terminal.
    Denied,
    /// A person must approve.
    ApprovalRequired,
    /// A valid, unexpired approval exists.
    Approved,
    /// Execution begun.
    ExecutionStarted,
    /// Executor reported success.
    Succeeded,
    /// Executor reported failure.
    Failed,
    /// Timed out after the request may have been accepted; must be reconciled before retrying.
    OutcomeUnknown,
    /// Remote outcome checked.
    Reconciled,
    /// Terminal success.
    Completed,
    /// Terminal compensating outcome.
    Compensated,
}

/// Events that drive [`WorkflowState`]. Authorisation never comes from an event alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowEvent {
    /// Context was resolved.
    ContextResolved,
    /// A plan was drafted.
    PlanDrafted,
    /// The plan passed schema validation.
    SchemaValidated,
    /// Policy was evaluated and its context hash recorded.
    PolicyEvaluated,
    /// Policy denied the action.
    Denied,
    /// Approval was requested from a person.
    ApprovalRequired,
    /// A person approved; the approval is verified against the current context.
    Approved {
        /// The approval to verify.
        approval: Approval,
    },
    /// Execution started with the proposal's idempotency key.
    ExecutionStarted,
    /// The executor reported success.
    Succeeded,
    /// The executor reported failure.
    Failed,
    /// The executor timed out with an ambiguous outcome.
    TimedOut,
    /// The remote outcome was reconciled.
    Reconciled,
    /// Reconciliation found a completed action.
    Completed,
    /// A compensating action completed.
    Compensated,
}

/// Per-transition facts supplied by the caller at the moment the event occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionContext<'a> {
    /// Current time, used to expire approvals.
    pub now: Timestamp,
    /// Fingerprint of the proposal as it exists now.
    pub proposal_hash: &'a ProposalHash,
    /// Fingerprint of the policy context as it exists now.
    pub policy_context_hash: &'a PolicyContextHash,
}

/// Why a transition was refused. Every refusal is a denial, never an implicit allow.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TransitionError {
    /// The event is not defined for the current state.
    #[error("event {0} is not allowed in state {1:?}")]
    InvalidTransition(String, WorkflowState),
    /// The approval does not bind to this proposal or target circle.
    #[error("approval does not match this proposal")]
    StaleApproval,
    /// The approval has expired or is not yet valid.
    #[error("approval is outside its validity window")]
    ApprovalOutsideWindow,
    /// The approval window itself is invalid.
    #[error("approval expires before or at the time it was given")]
    InvalidApprovalWindow,
    /// Permissions, target or consent changed after policy evaluation.
    #[error("policy context changed since evaluation; re-evaluate before proceeding")]
    PolicyContextChanged,
}

/// A proposal together with the state derived from it.
///
/// Fields are private so that an approval cannot exist without a matching state, and a policy
/// context hash cannot exist before evaluation. The only way to advance is [`Self::transition`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    id: ObjectId,
    state: WorkflowState,
    proposal: ActionProposal,
    policy_context_hash: Option<PolicyContextHash>,
    approval: Option<Approval>,
}

impl Workflow {
    /// Starts a workflow in `requested` with no evaluation and no approval.
    #[must_use]
    pub fn new(id: ObjectId, proposal: ActionProposal) -> Self {
        Self {
            id,
            state: WorkflowState::Requested,
            proposal,
            policy_context_hash: None,
            approval: None,
        }
    }

    /// Workflow identity.
    #[must_use]
    pub const fn id(&self) -> &ObjectId {
        &self.id
    }

    /// Current state.
    #[must_use]
    pub const fn state(&self) -> WorkflowState {
        self.state
    }

    /// The proposal being processed.
    #[must_use]
    pub const fn proposal(&self) -> &ActionProposal {
        &self.proposal
    }

    /// The approval, once one has been accepted.
    #[must_use]
    pub const fn approval(&self) -> Option<&Approval> {
        self.approval.as_ref()
    }

    /// The policy context hash recorded at evaluation time, if evaluation happened.
    #[must_use]
    pub const fn policy_context_hash(&self) -> Option<&PolicyContextHash> {
        self.policy_context_hash.as_ref()
    }

    /// Advances the workflow, or leaves it untouched and returns a denial.
    ///
    /// # Errors
    ///
    /// [`TransitionError`] if the event is invalid for the current state, the approval is
    /// stale or expired, or the policy context changed since evaluation.
    pub fn transition(
        &mut self,
        event: &WorkflowEvent,
        ctx: &TransitionContext<'_>,
    ) -> Result<(), TransitionError> {
        let current = self.state;
        let next = match (current, &event) {
            (WorkflowState::Requested, WorkflowEvent::ContextResolved) => {
                WorkflowState::ContextResolved
            }
            (WorkflowState::ContextResolved, WorkflowEvent::PlanDrafted) => {
                WorkflowState::PlanDrafted
            }
            (WorkflowState::PlanDrafted, WorkflowEvent::SchemaValidated) => {
                WorkflowState::SchemaValidated
            }
            (WorkflowState::SchemaValidated, WorkflowEvent::PolicyEvaluated) => {
                WorkflowState::PolicyEvaluated
            }
            (WorkflowState::PolicyEvaluated, WorkflowEvent::Denied) => WorkflowState::Denied,
            (WorkflowState::PolicyEvaluated, WorkflowEvent::ApprovalRequired) => {
                WorkflowState::ApprovalRequired
            }
            (
                WorkflowState::PolicyEvaluated | WorkflowState::ApprovalRequired,
                WorkflowEvent::Approved { approval },
            ) => {
                self.validate_approval(approval, ctx)?;
                WorkflowState::Approved
            }
            (WorkflowState::Approved, WorkflowEvent::ExecutionStarted) => {
                self.check_policy_context(ctx)?;
                WorkflowState::ExecutionStarted
            }
            (WorkflowState::ExecutionStarted, WorkflowEvent::Succeeded) => WorkflowState::Succeeded,
            (WorkflowState::ExecutionStarted, WorkflowEvent::Failed) => WorkflowState::Failed,
            (WorkflowState::ExecutionStarted, WorkflowEvent::TimedOut) => {
                WorkflowState::OutcomeUnknown
            }
            (
                WorkflowState::Succeeded | WorkflowState::Failed | WorkflowState::OutcomeUnknown,
                WorkflowEvent::Reconciled,
            ) => WorkflowState::Reconciled,
            (WorkflowState::Reconciled, WorkflowEvent::Completed) => WorkflowState::Completed,
            (WorkflowState::Reconciled, WorkflowEvent::Compensated) => WorkflowState::Compensated,
            _ => {
                return Err(TransitionError::InvalidTransition(
                    format!("{event:?}"),
                    current,
                ));
            }
        };

        match event {
            WorkflowEvent::PolicyEvaluated => {
                self.policy_context_hash = Some(ctx.policy_context_hash.clone());
            }
            WorkflowEvent::Approved { approval } => self.approval = Some(approval.clone()),
            _ => {}
        }
        self.state = next;
        Ok(())
    }

    fn validate_approval(
        &self,
        approval: &Approval,
        ctx: &TransitionContext<'_>,
    ) -> Result<(), TransitionError> {
        if approval.at >= approval.expires_at {
            return Err(TransitionError::InvalidApprovalWindow);
        }
        if ctx.now < approval.at || ctx.now >= approval.expires_at {
            return Err(TransitionError::ApprovalOutsideWindow);
        }
        if approval.proposal_hash != *ctx.proposal_hash
            || approval.target_circle != self.proposal.target_circle
        {
            return Err(TransitionError::StaleApproval);
        }
        self.check_policy_context(ctx)
    }

    fn check_policy_context(&self, ctx: &TransitionContext<'_>) -> Result<(), TransitionError> {
        if self.policy_context_hash.as_ref() != Some(ctx.policy_context_hash) {
            return Err(TransitionError::PolicyContextChanged);
        }
        Ok(())
    }
}
