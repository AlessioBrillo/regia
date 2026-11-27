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

None for the decision itself. Schema v0 (`regia-domain`, `Envelope`) is a logical wire contract that is
independent of storage: it carries no revision, clock or merge metadata, so it does not pre-empt this ADR or ADR-003.
Benchmark plan and synthetic dataset: [DAT-01](../experiments/DAT-01-data-model-sync.md) (planned, not executed).

Increment 3 ([results](../experiments/dat-01/results-increment-3.md)): migration (S-06), long backlog (S-08) and alternative C (hybrid) measured. No gate listed in the card rejects an alternative; the frozen S-08 byte criterion is missed by all three, so it cannot choose, and its meaning is an owner decision. The Decision stays pending.

Schema v0 amendment (2026-11-27): the owner decision that an *area* is where an object is filed and a *circle* is who can see it is now a logical type (`Area { id, name, parent }`) and an optional `Fact<AreaId>` on `ObjectHeader`. A model-proposed area must carry a source; a person's confirmed choice may carry an unsourced reason. This changes the wire contract of the logical schema only (fixtures and contract suite updated); it chooses no storage, revision or merge representation, so ADR-002 stays `proposed`.

## Consequences

To be filled when the decision is taken.

## Migration / rollback

To be filled when the decision is taken.

## Residual risks

To be filled when the decision is taken.

## Conditions to reopen

To be filled when the decision is taken.
