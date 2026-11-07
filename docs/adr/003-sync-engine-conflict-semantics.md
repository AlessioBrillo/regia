# ADR-003 — Sync engine and conflict semantics

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-02, AR-03, R-01, R-02

## Context

Sync is an application protocol (identity, causality, tombstones, schema versioning, replay, backpressure, policy at fetch and at materialization). A CRDT converges replicas but does not authorize edits or encode domain rules (report §4.4).

## Alternatives considered

A. Automerge sync.
B. Relational sync with server-ordered operations.
C. Hybrid per object type.

## Evaluation method

Benchmark DAT-01 and workflow WF-04 scenarios (offline reorder, revocation while offline). Merge rules per data type: checklist set, collaborative text, calendar interval, approval transition.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: not every field is a natural CRDT; explicit conflicts are allowed where a silent choice would lose intent.

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
