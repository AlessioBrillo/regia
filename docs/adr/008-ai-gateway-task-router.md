# ADR-008 — AI gateway and task router interface

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-09, AR-10, R-07

## Context

A common `TaskRequest` / `TaskResult` contract with data classification, allowed scope, capabilities, latency budget, offline requirement and recorded model/provider/version (report §4.8). A fallback must never silently change data treatment.

## Alternatives considered

A. Thin adapter per runtime behind a versioned schema.
B. Generic universal adapter up front.

## Evaluation method

Implement at least two real runtimes before generalizing; contract tests per adapter; routing decisions audited without prompt content.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: no over-abstract adapter before two real runtimes exist.

## Evidence

None yet.

## Consequences

To be filled when the decision is taken.

## Migration / rollback

To be filled when the decision is taken.

## Residual risks

To be filled when the decision is taken.

## Conditions to reopen

To be filled when the decision is taken.
