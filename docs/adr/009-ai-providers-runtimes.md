# ADR-009 — On-device and cloud AI providers and runtimes

- Status: proposed
- Date: 2026-11-07
- Owner: Principal Engineer (area owners per [GOVERNANCE](../../GOVERNANCE.md))
- Requirements: AR-09, AR-10, R-07, R-08

## Context

Candidates: Apple Foundation Models, ML Kit / Gemini Nano, open-weight runtimes (llama.cpp, ONNX Runtime) and cloud providers in the EU or global. No runtime is assumed available on every device.

## Alternatives considered

Per task family (report §4.8 table): rules and small classifiers, OCR + structured extraction, cited Q&A, planning, sensitivity classification, voice, routine forecasting.

## Evaluation method

Golden datasets per task, including easy, ambiguous, adversarial and out-of-distribution cases. Measure correctness, dangerous errors, citations, p50/p95 latency, cost and resources per correct result, memory, energy, offline operation and provider retention.

## Decision

**Pending.** Not ratified before the evaluation above produces reproducible evidence (report §14.2).

Constraint: publish a device/OS support matrix and a deterministic fallback.

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
