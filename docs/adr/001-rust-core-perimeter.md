# ADR-001 — Perimeter of the shared Rust core

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-01, AR-13, R-14

## Context

Which logic is worth sharing between iOS and Android in Rust (report §4.1–4.2)? Candidate crates: `regia-domain`, `regia-policy-adapter`, `regia-sync-adapter`, `regia-provenance`, `regia-action-contracts`, `regia-crypto-adapter`. Only `core/domain` exists today.

## Alternatives considered

A. Fully native logic on each platform (duplication).
B. Selective Rust core for high-consistency logic with native shells (the report's provisional recommendation).
C. Rust for everything shareable, including integrations.

## Evaluation method

Measure binding cost (UniFFI/C), crash debugging, binary size, development velocity and tests on iOS and Android within experiment CL-01 (§4.1).

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: no one-line wrapper crates; split `core/domain` only when reuse or isolation is demonstrated.

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
