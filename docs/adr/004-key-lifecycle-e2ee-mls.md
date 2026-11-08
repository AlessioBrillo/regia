# ADR-004 — Key lifecycle, E2EE and MLS

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-05, R-02, R-09

## Context

Group key agreement for circles, member add/remove, device add, rotation, offline-at-revocation, backup and recovery. MLS (RFC 9420) is a candidate; it does not solve identity, recovery, application authorization or persisted-object encryption (report §4.5).

## Alternatives considered

A. MLS via a maintained, verified implementation.
B. Per-circle symmetric keys with an audited key-wrapping scheme.
C. Server-managed encryption for some labels only.

## Evaluation method

Spike CRY-01: create circle, invite, accept, add device, remove member, rotate, in-flight messages, persisted data, offline at revocation, backup, restore. Distinguish future access, historical data, previously downloaded copies and residual metadata.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: no proprietary cryptography. Document where plaintext exists (device, RAM, relay, orchestrator, logs, AI provider, OCR/STT, object store, backups).

## Evidence

None yet. Experiment plan: [CRY-01](../experiments/CRY-01-group-keys-membership.md) (planned, not executed).

## Consequences

To be filled when the decision is taken.

## Migration / rollback

To be filled when the decision is taken.

## Residual risks

To be filled when the decision is taken.

## Conditions to reopen

To be filled when the decision is taken.
