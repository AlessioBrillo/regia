# ADR-002 — Data model and source of truth

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-01, AR-03, AR-11, R-01, R-14

## Context

For each object type: is the single source of truth a CRDT document, a SQL table or a domain event log? How are local indexes kept consistent and rebuilt (report §4.3)?

## Alternatives considered

A. Versioned SQLite relational schema.
B. Automerge documents for collaborative objects.
C. Relational storage with a dedicated sync engine.
D. Hybrid with a declared authority per datum.

## Evaluation method

Benchmark DAT-01: 2 and 5 clients, offline edits, concurrent edits, delete/restore, schema migration, crash during write, circle isolation. Security and consistency gates pass first; only then compare latency and cost.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: Automerge + SQLite is a candidate, not a decision.

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
