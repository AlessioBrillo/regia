# ADR-012 — Native clients versus cross-platform

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-02, AR-14, R-08

## Context

iOS SwiftUI + Android Compose (provisional) versus Flutter / React Native, with an optional Rust core (report §4.1).

## Alternatives considered

A. Native SwiftUI + Compose.
B. Flutter or React Native.
C. Rust core + native shells.

## Evaluation method

Spike CL-01: share-sheet text/PDF → persist locally → structured proposal → confirm/cancel → sync → network-error handling. Compare effort, stability, accessibility, perceived latency, storage and debuggability. Pick the first platform from the devices actually available.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: never claim multi-platform support before a tested build.

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
