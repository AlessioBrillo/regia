# Experiment card

Copy to `docs/experiments/<ID>-<slug>.md`. Every experiment yields a repeatable artifact (code, fixtures, run
instructions, raw measurements, results, open defects, decision, ADR). "Works on my device" is not evidence.

```text
ID / version / owner:
Technical question:
Linked vision requirement:
Initial hypothesis and alternatives:
Relevant threat model:
Reproducible setup (OS, device, versions, model, dataset):
Method and test inputs:
Metrics collected and units:
Raw results and repetitions:
Failed cases / limits / anomalies:
Effect on requirements and other components:
Decision: adopt / trial / defer / reject
Linked ADR:
Conditions that would reopen the decision:
```

## Scoring weights (frozen — change only before results are seen)

Alternatives first pass the minimum requirements; security is a **gate**, not an averaged criterion.

| Criterion | Weight |
|---|---:|
| Security, privacy and isolation | 20% |
| Functional correctness and preservation of intent | 15% |
| Offline, convergence and recovery | 15% |
| AI quality per task and citation reliability | 10% |
| End-to-end latency | 10% |
| Operability and failure diagnosis | 10% |
| Portability and lock-in | 5% |
| Multi-platform interoperability | 5% |
| Resources and cost per successful workflow | 5% |
| Simplicity of evolution and maintenance | 5% |
| **Total** | **100%** |

## Planned experiments

`CL-01` client comparison · `DAT-01` data model and sync · `CRY-01` group keys and membership · policy engine spike
(Cedar vs OPA/Rego) · `WF-01` (§4.7) state machine vs Temporal · AI evaluation harness per task family.
