# ADR-010 — Telemetry and audit

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-12, R-11

## Context

OpenTelemetry is the candidate for correlating traces, metrics and logs, configured so personal payloads never become telemetry. Audit records decisions, sources, actor, policy version and result with minimization (report §4.12).

## Alternatives considered

A. OpenTelemetry with redaction by default.
B. Custom event log only.

## Evaluation method

Log and trace leak tests (no text, names, documents, prompts, tokens); pseudonymous rotating identifiers; access control and retention of audit data.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: audit access is itself authorized and auditable.

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
