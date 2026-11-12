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

**Proposed, awaiting owner acceptance:** trial **Cedar** as the policy engine, keeping the hand-written oracle as a permanent differential-test fixture. Alternative C is not needed: both A and B passed the gate. Not ratified until the owner accepts this ADR on the PR (report §14.2).

Constraint: engine unreachable or attribute missing means deny for high-risk actions; an error is never mapped to allow.

## Evidence

[POL-01 results](../experiments/pol-01/results-2026-11-12.md), code in `spikes/pol01`: both engines pass P-01..P-11 and agree with the oracle on 10 000 random cases; every error path denies. Cedar: strict schema validation rejects unknown attributes before evaluation, 89 vs 162 crates, p50 ≈ 57 vs 84 µs. Rego (regorus): simpler transitive-ancestor rule, but no static validation (a misspelt field in a `deny` silently disables it).

## Consequences

If accepted: policies live in Cedar with a schema and CI validation; the application layer builds the minimal entity graph (ancestor groups, revoked groups); any evaluation error is treated as deny (Cedar skips an erroring policy, and a skipped `forbid` would fail open); the oracle stays as a test fixture.

Nested-circle rule fixed by the owner for this spike (P-03): only an `Owner` of an ancestor circle may read/discover child-circle objects labelled `circle`; `Member`/`Guest` of the parent get nothing; `restricted` and `secret` are never inherited. The label names and roles remain provisional until this ADR is accepted.

## Migration / rollback

No production policy exists yet. The request model and the oracle are engine-neutral, so switching to Rego (or the oracle) means re-implementing one adapter and the policy file; the case table and property test are the acceptance suite.

## Residual risks

iOS/Android bindings and local execution unmeasured (CL-01). OPA proper (not regorus) untested. Rules authored by one person and not independently reviewed. Cedar has a single implementation. Transitive-rule expressiveness needs adapter-side graph construction.

## Conditions to reopen

A bypass in Cedar semantics; Cedar cannot run on a target device (CL-01); a policy class the model cannot express; OPA proper shown materially better on rule review or operability.
