# DAT-01 results, increment 3 — 2026-11-20

S-06 (migration v0 to v1), S-08 (long offline backlog) and alternative C (hybrid) on the rules frozen in
[the card](../DAT-01-data-model-sync.md#rules-fixed-for-increment-3-2026-11-20-before-any-result) before any run. Code:
[`spikes/dat01`](../../../spikes/dat01). Raw output: [`bench-raw-increment-3.txt`](bench-raw-increment-3.txt).
Earlier increments: [1](results-2026-11-13.md), [2](results-increment-2.md).

```text
cargo test -p regia-spike-dat01
cargo run -p regia-spike-dat01 --example bench   --release -- 10
cargo run -p regia-spike-dat01 --example backlog --release -- 10 1000
```

## Setup (recorded, not assumed)

| Item | Value |
|---|---|
| Alternatives | A `automerge` 0.12.0; B `rusqlite` 0.42.0 (bundled SQLite, JSON wire); C = A for `kind` `document`/`note`, B for everything else, binary framing of the two sync messages |
| Toolchain | `rust:1.98.1` container (the CI image), `--release` for the runs |
| Host | Intel Core 7 150U, Windows 11, container under WSL2 (5 GB VM, 2 build jobs) |
| Load on the host | **Shared.** Other containers (another project) were running during some runs. Times below are noisy; byte counts are deterministic |
| Workload | Synthetic only. Every create carries a `kind` field for all three alternatives, so numbers differ from increment 1 and are only comparable within this run |

## Gate (pass/fail, not averaged)

| Check | A | B | C |
|---|---|---|---|
| S-06 v0 shows the object, flags "update the app", hides `area`, refuses to write it | pass | pass | pass |
| S-06 `area` written on v1 crosses a v0 device unchanged (device 2 learns it only through v0) | pass | pass | pass |
| S-06 concurrent edits across the version boundary: both kept, v1 replicas equal | pass | pass | pass |
| S-06 conflict on `area` visible to v1, hidden from v0 (flag only), cleared when v1 writes again | pass | pass | pass |
| S-06 canary scan | 0 leaks | 0 leaks | 0 leaks |
| S-08 convergence, no silent loss, canary scan (10 seeds x 5 clients x 1000 ops) | pass | pass | pass |
| S-08 threshold: in-process convergence p95 < 5 s | **miss** (11.2 s) | pass (4.0 s) | pass (4.0 s) |
| S-08 threshold: bytes <= 10x the net payload | **miss** (15.2x) | **miss** (31.6x) | **miss** (24.3x) |
| Scenarios S-01..S-05, S-09, S-10, S-11 and the differential test A = B = C (96 random histories) | pass | pass | pass |
| S-07, S-12 (real process kill, several processes) | pass (inc. 2) | pass (inc. 2) | **not run** |

S-08 detail (10 seeds, the p95 of ten values is the largest):

| | convergence ms, min / median / max | bytes on the wire (seed range) | payload | bytes / payload |
|---|---|---|---|---|
| A | 2 482 / 4 436 / 11 230 | 1 385 652 – 1 388 775 | about 91 300 B | 15.2x |
| B | 1 702 / 2 243 / 4 022 | 2 881 794 – 2 882 122 | about 91 300 B | 31.6x |
| C | 1 628 / 2 663 / 4 002 | 2 133 034 – 2 218 974 | about 91 300 B | 23.4x – 24.3x |

**Every alternative misses the frozen byte criterion, and A also misses the time criterion.** The thresholds are not
changed (the rule says a miss is reported as a miss). What the miss means is a question for the owner, see "Outcome".

## Sensitivity (the checks can fail)

Each mutation was applied to the committed source, the tests run, and the source restored.

| Mutation | Result |
|---|---|
| v0 allowed to write `area` | Caught: `s06_v0_flags_and_refuses` fails on A and B |
| v0 does not hide v1 fields | Caught: 6 tests fail (3 scenarios, A and B) |
| v0 does not flag "update the app" | Caught: 4 tests fail |
| C: a failed object lookup is cached as "side B" (the bug found while writing C) | Caught by the differential test A = B = C (minimal input: a `Set` on an unknown object, then its create arrives, then a `Delete`). Fixed; the same input now passes |
| C: `Revoke` not applied on side A | Caught: `s09_revoked_offline_edits_are_quarantined` fails on C. Only that one test, because only some objects live on A |

"v0 forwards `area` unchanged" is not mutation-tested: both stores keep unknown fields by construction, so there is no
branch to break.

## Metrics (bench, median over seeds 1..=10)

| Alt, profile, devices | write p50 / p95 (us) | sync ms med / p95 | bytes on the wire | storage per op (B) | index rebuild (ms) |
|---|---:|---:|---:|---:|---:|
| A small, 5 | 101 / 334 | 36 / 48 | 49 513 | 157 | 0.6 |
| B small, 5 | 69 / 149 | 8 / 15 | 46 364 | 1 886 | 0.3 |
| C small, 5 | 119 / 371 | 25 / 44 | 63 655 | 1 465 | 0.4 |
| A large, 5 | 178 / 401 | 93 / 258 | 32 139 | 45 | 18.5 |
| B large, 5 | 240 / 609 | 28 / 91 | 122 270 | 706 | 4.4 |
| C large, 5 | 240 / 544 | 60 / 282 | 95 193 | 493 | 9.0 |

Every cell kept 100 % of intent and surfaced every forced conflict (36/36 small, 150/150 large); the 2-device rows are
in the raw file (the large 2-device row is degenerate, as in increment 1). Adapter size (non-blank, non-comment
lines, including the unit tests in the file): A 539, B 612, C 178. C is small because it owns no storage and no merge
logic; it reuses A and B, so it is the sum of both in everything else.

Write latency is higher than in increment 1 on the same code path; the host was shared, so do not read the
difference as a regression. Local write p95 stays far under the owner's 50 ms target (worst: 0.61 ms).

## Findings that matter for the ADRs

1. **The byte criterion does not separate the alternatives.** All three miss it. B's ratio is mostly its untuned JSON
   wire with a full version vector per op (finding 3 of increment 1), so it measures the format, not the design. A is
   the closest (15.2x) with a binary format. No claim on design efficiency until B has a binary encoding.
2. **A's convergence time has a heavy tail.** Median 4.4 s, but one seed took 11.2 s; the other two alternatives stay
   under 4.1 s. With a shared host this is not proof, but it agrees with increment 2, finding 2 (A replays the
   circle document). Worth re-measuring on a quiet machine before relying on it.
3. **C inherits the weaknesses of both stores.** Revocation on the A side still needs the replay and holds dependent
   edits aside (increment 2, finding 1); the B side honours it per op. A hybrid therefore has two revocation
   behaviours inside one circle. C also commits in two places: a `Revoke` is two writes, and a frame whose A half is
   accepted and B half refused is applied half. Neither is tested for a crash between the two.
4. **A hybrid needs a routing rule that survives sync.** The differential test found that the first version cached a
   wrong side for an object a peer had not created yet. The fix is small, but it shows the cost of C: every new
   object type needs a decision on which store holds it, forever (an object never changes side).
5. **Migration (S-06) is cheap on both stores.** Both keep unknown fields as ordinary data, so a v0 client forwards a
   v1 field without code. The work is in the read path (hide, flag) and the write guard. This does not test a
   rename, a type change or a removed field.

## Failed cases, limits, anomalies

- S-08 runs in one process with no network; times are a lower bound. The 1000-op backlog is a synthetic mix (about 10 %
  creates, 10 % edits of one shared field that force conflicts, the rest status edits of own objects).
- S-07 and S-12 were not run on C. C has no storage of its own, so they remain evidence of A and B, but a crash
  between C's two commits is untested.
- Timings were taken on a host shared with other workloads. Rerun on a quiet machine before quoting a time.
- The raw file ends with the panic of the S-08 example: it asserts the thresholds, and they were missed.
- Not covered: alternative C for concurrent moves, re-admission, signed authorship, encryption (CRY-01), policy
  (POL-01); the limits of increments 1 and 2 not named above still stand.

## Scoring (frozen weights, [`TEMPLATE.md`](../TEMPLATE.md))

All three pass the isolation gate (0 leaks) and converge without silent loss. Only measurable criteria are compared.

| Criterion (weight) | A | B | C | Basis |
|---|---|---|---|---|
| Security, privacy, isolation (20%) | equal | equal | equal | 0 leaks each, positive control in place |
| Functional correctness / intent (15%) | equal | equal | equal | all scenarios and the differential test pass |
| Offline, convergence, recovery (15%) | worse | better | between | S-08 time; A tail; revocation per op (B) vs replay (A) |
| End-to-end latency (10%) | about equal | about equal | about equal | sub-millisecond writes; sync time is CPU only |
| Operability / diagnosis (10%) | not measured | not measured | not measured | |
| Simplicity of evolution (5%) | more library surface | all logic visible | two stores to reason about | judgement |
| Resources (5%) | better storage and wire | worse (untuned JSON) | between | S-08 bytes, storage per op |
| Portability, multi-platform (10%), AI (10%) | not measured | not measured | not measured | CL-01 |

No numeric total: a third of the weight is still unmeasured.

## Outcome

**No ADR is accepted by this PR.** What the evidence supports:

- None of A, B, C is rejected by a gate listed in the card (isolation, convergence without silent loss).
- The S-08 byte criterion, as frozen, would reject all three, so it cannot be used to choose. Whether it is a gate or
  a metric, and whether 10x is the right figure, is the owner's call. I did not change it.
- C does not beat both parents on any measured criterion, and adds routing, a second commit and two revocation
  behaviours. On this evidence the engineer's lean (judgement, not a result) is to keep C as a fallback and decide
  between A and B. B keeps revocation and migration logic visible and per op; A is smaller on disk and on the wire
  but needs the replay and the dependent-edit rule. Both need CL-01 (can the library run on iOS, Android and Windows)
  before ADR-002/003 can close.

ADR-002 and ADR-003 stay `proposed`; their Evidence sections now point here.
