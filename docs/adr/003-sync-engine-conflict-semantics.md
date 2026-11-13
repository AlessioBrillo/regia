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

Partial, increment 1 of [DAT-01](../experiments/DAT-01-data-model-sync.md): [results 2026-11-13](../experiments/dat-01/results-2026-11-13.md). Both alternatives met the spike rules (S-03 lww with visible conflict, S-05 resurrect with notice) and the isolation gate; no recommendation yet. The conflict rules were fixed by the owner for the spike only (2026-11-12) and are not ratified here. Revocation while offline (S-09), migration, backlog and alternative C are increment 2.

## Consequences

To be filled when the decision is taken.

## Migration / rollback

To be filled when the decision is taken.

## Residual risks

To be filled when the decision is taken.

## Conditions to reopen

To be filled when the decision is taken.
