# Architecture requirements (AR-01 … AR-15)

Source: technical report v1.0 §2. Status values: `open` (no evidence yet), `partial`, `verified` (reproducible
evidence stored). Every requirement needs a verification method and an owner before Gate G0 closes.

| ID | Property | Evidence required | Verification method | Owner | Status |
|---|---|---|---|---|---|
| AR-01 | Explicit domains and contracts | Versioned schema, API contracts, tested migrations | Contract + migration tests | TBD | partial (`regia-domain` types; entity schema v0 + contract suite, no migrations yet) |
| AR-02 | Real local-first | Operations in airplane mode; later sync verified | E2E offline scenarios (WF-04) | TBD | open |
| AR-03 | Deterministic convergence | Property-based tests, network-partition simulation | DAT-01 benchmark | TBD | open |
| AR-04 | Circle boundaries enforced everywhere | Negative tests on API, sync, search, cache, notifications, export, prompt, audit | Security test suite | TBD | open |
| AR-05 | Encryption consistent with the threat model | Threat model, key lifecycle, revocation/rotation/recovery tests | CRY-01 spike | TBD | partial (threat model baseline, `docs/threat-model`; no evidence yet) |
| AR-06 | Knowledge provenance | Every extracted fact keeps source, position, timestamp, confidence, confirmation state | Type-level invariants + automated test | TBD | partial (`Provenance`, `Fact` with confidence; enforced on every entity by the v0 contract suite) |
| AR-07 | Governed agent | Every mutating tool call has a policy decision, approval, state and correlated audit | Policy + action tests | TBD | open |
| AR-08 | Resumable workflows | Crash/timeout injection; resume without duplicate side effects | Fault injection (WP5) | TBD | open |
| AR-09 | Replaceable AI providers | Common interface, contract tests, per-task benchmarks | Adapter contract tests | TBD | open |
| AR-10 | Explicit degradation | Recognisable UI state; no false success; safe non-AI alternatives | UX + failure tests | TBD | open |
| AR-11 | Data portability | Verified export/import; deletion tests; projection consistency | Round-trip + deletion tests | TBD | open |
| AR-12 | Observability without exfiltration | Correlated traces without personal payloads by default; controlled audit access | Log-leak tests | TBD | open |
| AR-13 | Modular evolution | Bounded contexts, internal APIs, data ownership, backward-compatible migrations | Review + migration tests | TBD | open |
| AR-14 | Different experiences, common rules | Same policies and source of truth for all profiles | Cross-surface policy tests | TBD | open |
| AR-15 | Verifiable security | Threat modeling, abuse suite, isolation tests, independent review before widening trials | Security review gate | TBD | partial (threat model baseline; review not done) |

## Statements that need a precise specification before implementation (report §2.1)

- **"Zero loss"** must declare which operations, data types and conflicts are covered; verify each user intention in a
  scenario matrix.
- **"Immediate revocation"** stops future access via server and updated keys; it cannot delete copies or information
  already seen. UX and threat model must say so.
- **"E2EE and cloud AI"** are compatible only if it is stated which content is decrypted, by whom, where, for what and
  with what consent.
- **"Sensitivity classification recall ≥ 95%"** needs a taxonomy, a representative dataset, false-negative handling and
  a fail-safe policy; a model classification must never be the only control for sending data to a cloud.
- **"99.9% critical notification SLO"** needs the measured event, dependencies, window, exclusions and behaviour when
  push/telephony are outside Regìa's control.
- **"Data in the EU"** must be verified per provider, sub-processor, log, support, backup, inference and telemetry.
