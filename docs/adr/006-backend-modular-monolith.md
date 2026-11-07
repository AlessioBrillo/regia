# ADR-006 — Modular monolith, storage and backend API

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-13, R-11

## Context

Backend baseline: versioned HTTPS API, modular monolith, PostgreSQL for server-authorized data, EU S3-compatible object storage, KMS for server secrets, transactional outbox, tested backup/restore, no personal content in logs (report §4.6).

## Alternatives considered

A. Modular monolith.
B. Early microservices.
C. Serverless functions.

## Evaluation method

Architecture review plus failure-injection results from the vertical slice. Split a service only for measured scale, isolation, reliability or incompatible-library reasons.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: per object, declare server-readable, end-to-end encrypted or transient-worker (see `ServerVisibility`).

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
