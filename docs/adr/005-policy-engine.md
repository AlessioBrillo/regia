# ADR-005 — Policy engine and label taxonomy

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-04, AR-07, AR-15, R-03

## Context

Deterministic, deny-by-default authorization over principal/action/resource/context, with explainable decisions, outside any model. The confidentiality label names in `regia-domain` are provisional and are fixed here (report §4.5).

## Alternatives considered

A. Cedar.
B. OPA/Rego.
C. Hand-written evaluator in Rust (only if A and B fail the spike).

## Evaluation method

Spike with nested Regìa policies, generated tests, decision explainability, versioning, performance, backend bindings and local execution. The application layer prepares a minimal, complete input.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: engine unreachable or attribute missing means deny for high-risk actions; an error is never mapped to allow.

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
