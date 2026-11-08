# POL-01 — Policy engine spike (Cedar vs OPA/Rego)

Status: **plan, not executed.** Report §4.5. Feeds ADR-005. Owner: TBD ([#17](https://github.com/AlessioBrillo/regia/issues/17)).

```text
ID / version / owner:            POL-01 / v1 (plan) / TBD
Technical question:              Which engine expresses Regìa's nested-circle authorisation with explainable,
                                 deny-by-default decisions that behave identically in backend, sync, search and
                                 on device?
Linked vision requirement:       AR-04, AR-07, AR-14, AR-15; R-03
Initial hypothesis/alternatives: A  Cedar
                                 B  OPA/Rego
                                 C  Hand-written Rust evaluator (only if A and B fail the gate)
                                 No alternative is preferred.
Relevant threat model:           T-03, T-04, T-06, T-12 (docs/threat-model)
Reproducible setup:              Engine + exact version, OS, hardware, policy and input fixtures in the repo
Method and test inputs:          Principal/action/resource/context model below; DAT-01 dataset as resources
Metrics collected and units:     See "Metrics"
Raw results and repetitions:     Not executed
Failed cases / limits:           Not executed
Effect on requirements:          Not executed
Decision: adopt/trial/defer/reject:  Not executed
Linked ADR:                      ADR-005
Conditions that would reopen:    A bypass found in the chosen engine's semantics; need for local execution that
                                 the engine cannot satisfy; a policy class the model cannot express
```

## Model under test

- **Principal:** person, device, service identity. **Action:** read, discover existence, create, update, delete,
  export, share, execute. **Resource:** object plus related objects. **Context:** circle, role, label, purpose,
  channel, autonomy level, consent state, time when relevant (§4.5).
- The application layer builds a **minimal, complete input**; the engine never fetches data on its own.
- Label taxonomy (`public`/`circle`/`restricted`/`secret` in `regia-domain` are provisional) is fixed by this spike's
  outcome in ADR-005.

## Cases

| ID | Case | Expected |
|---|---|---|
| P-01 | Member reads object in own circle | allow |
| P-02 | Member reads object in a sibling or parent circle | deny |
| P-03 | Nested circle: inheritance and override rules | as declared, tested both ways |
| P-04 | `restricted` and `secret` objects | only named principals / owner |
| P-05 | "Discover existence" distinct from "read" | no title, count or snippet leak |
| P-06 | Missing attribute, unknown label, engine unreachable, policy error | **deny** (high-risk); never allow |
| P-07 | Mutating tool call proposed by an agent from untrusted content | requires approval; decision logged |
| P-08 | Policy version change mid-workflow | decision bound to the evaluated version |
| P-09 | Revoked member, stale local copy | deny at fetch and at materialisation |
| P-10 | Generated tests: random principal/resource/context triples vs an oracle | zero disagreements |

## Metrics

| Metric | Unit |
|---|---|
| Disagreements with the oracle (P-10) | count (must be 0) |
| Error and missing-attribute behaviour (P-06) | pass/fail; any `allow` is a gate failure |
| Decision latency p50/p95 for the cases above, backend and on device | ms |
| Explainability: can the decision name the rule and inputs that produced it | pass/fail per case |
| Policy versioning, rollout, rollback | qualitative + steps required |
| Bindings: backend (Rust), iOS, Android, local execution | available / effort |
| Authoring effort and readability by a reviewer | hours; reviewer notes |

## Gate and scoring

1. **Gate first:** P-06 passes in every error path; zero oracle disagreements; no case lets a model decide or
   override a decision.
2. Survivors scored with the frozen weights in [`TEMPLATE.md`](TEMPLATE.md).
3. The engine does not replace enforcement at every surface; AR-14 (same policy for all profiles) is checked by
   running the same case table through each surface.

## Out of scope

Key management (CRY-01), AI routing policy details (ADR-008).
