# ADR-007 — State machine and outbox versus Temporal

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-07, AR-08, R-06

## Context

Workflows span milliseconds to days: approvals, timers, retries, external side effects, ambiguous outcomes (`outcome_unknown`) (report §4.7, §5.3).

## Alternatives considered

A. Persistent state machine + transactional outbox + workers.
B. Temporal or an equivalent durable-execution runtime.

## Evaluation method

Spike WF-01 (report §4.7, not to be confused with workflow WF-01 of §5.1): input → extraction → approval → simulated external side effect → timeout → retry → reconciliation, on both. Compare recovery, timers, human waits, versioning, payload handling and operational cost.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: idempotency of external effects is required with either choice.

## Evidence

None yet. Experiment plan: [WFE-01](../experiments/WFE-01-workflow-runtime.md) (planned, not executed).

## Consequences

To be filled when the decision is taken.

## Migration / rollback

To be filled when the decision is taken.

## Residual risks

To be filled when the decision is taken.

## Conditions to reopen

To be filled when the decision is taken.
