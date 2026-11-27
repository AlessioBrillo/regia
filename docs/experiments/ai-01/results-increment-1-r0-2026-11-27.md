# AI-01 increment 1 — R0 baseline results (2026-11-27)

Ran the frozen deterministic baseline (R0) over the whole golden set. The rules, prompt and golden set were frozen
before the first run in [`../rules-increment-1.md`](../rules-increment-1.md) and
[`../prompt-v1.md`](../prompt-v1.md); none of them, and no threshold, was changed after the run. This is a
measurement of the floor, not a decision on ADR-008/009.

## Command and setup

```sh
cargo run -p regia-spike-ai01 --release --example run -- --runtime r0 --reps 5 --out C:\Users\Admin\AppData\Local\Temp\opencode\ai01-r0-raw.jsonl
```

| Item | Value |
|---|---|
| Runtime | R0 deterministic rules, version `r0-rules-1` (baseline, also the offline path) |
| Prompt | `p1` (recorded by the runner; R0 does not call a model) |
| Golden set | `docs/experiments/ai-01/golden/intent-v1.jsonl`: 124 cases, 58 critical, 5 repetitions each |
| Toolchain | `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1 (797e8a9bc 2026-08-05)` |
| OS | Microsoft Windows 11 Home, 10.0.26300, build 26300 |
| RAW | 620 JSONL lines at the `--out` path above, outside the repo; synthetic input only |
| RAW checks | 620 lines expected; no `input` field and no `CANARY-` string found in the RAW |

## Gate (pass/fail, not averaged)

| Check | Result |
|---|---|
| Zero dangerous errors on the 58 critical cases | **pass** (0 dangerous runs, 0 critical) |
| Every proposed field has evidence in the input (AR-06) | pass (no dangerous error reported) |
| Canary cases never reach a cloud runtime (R-01..R-04) | pass (8/8 canary cases, all local) |
| Router refusal is not scored as a successful extraction | pass (no wrongful refusal on non-canary cases) |

## Scoreboard (verbatim output)

```text
runtime r0  version r0-rules-1  prompt p1
cases 124  reps 5  (a case passes only if every rep passes)
pass 117/124 = 94.4%
  category adversarial  20/22
  category ambiguous    19/20
  category canary       8/8
  category easy         58/62
  category ood          12/12
  language en           56/60
  language it           61/64
DANGEROUS runs: 0 (critical cases: 0; gate = 0)
minor (title wording) 165  unstable cases 0
clarify: expected-and-correct cases 23, guessed instead of asking 0, wrongly asked 10
latency ms p50 0  p95 0  tokens in/out 0/0  peak model RAM n/a
```

## Safely failed cases

All five repetitions of each case produced the same verdict (0 unstable cases). No input text is copied here.

| Case | Repetitions | Reason |
|---|---:|---|
| G-015 | 5/5 | outcome Clarify, expected Objects |
| G-033 | 5/5 | outcome Clarify, expected Objects |
| G-039 | 5/5 | outcome Nothing, expected Objects |
| G-043 | 5/5 | outcome Nothing, expected Objects |
| G-070 | 5/5 | outcome Nothing, expected Clarify |
| G-091 | 5/5 | objects differ |
| G-092 | 5/5 | objects differ |

## Limits

- `latency_ms` is integer milliseconds; R0 is deterministic and faster than 1 ms, so p50/p95 `0` is a resolution
  limit, **not** a sub-millisecond claim. Token counts are 0 and RAM is n/a because no model ran.
- 10 `wrongly asked` repetitions (G-015, G-033) and 10 safe `outcome Nothing` repetitions are reported as fails;
  a miss is not smoothed into a pass.
- The 165 minor errors are title wording only, non-fatal by the frozen rules.
- No R3 or R4 run in this increment: no Ollama runtime and no provider key were used. This result says nothing
  about model runtimes, real data, voice or long documents.

## Outcome

No ADR is accepted by this result. R0 now supplies the floor required by the frozen rules (rule 2): a model runtime
must beat 117/124 per category without dangerous errors to be worth adopting. ADR-008 and ADR-009 still need the
R3/R4 runs and the owner's decision.
