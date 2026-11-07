# ADR-011 — Search and index strategy

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-04, AR-11, R-09

## Context

Order: deterministic queries and access filters, then local full-text (SQLite FTS5), then semantic indexing, then hybrid search only after a recall benchmark (report §4.9). Vectors are derived data: rebuildable and deletable together with their sources.

## Alternatives considered

A. SQL + FTS5 only.
B. SQL/FTS plus a local vector index.
C. Dedicated search engine.

## Evaluation method

Benchmark SQL/FTS first. Retrieval tests must include queries the user is not entitled to answer: the correct result is no disclosure.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: filter by circle and confidentiality before results, snippets or model context.

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
