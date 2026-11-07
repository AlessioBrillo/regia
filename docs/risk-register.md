# Risk register

Source: technical report v1.0 §11. Probability and impact are initial estimates; review at every gate.

| ID | Risk | Prob. | Impact | Early indicator | Mitigation / decision | ADR |
|---|---|---|---|---|---|---|
| R-01 | CRDT + SQL + E2EE complexity becomes unsustainable | High | Critical | Diverging indexes, odd merges, manual recovery | Spike before consolidating; one source of truth per object | 002, 003 |
| R-02 | Member removal does not cover offline devices and historical copies | Med-high | Critical | Persistent access after revocation; ambiguity on historical data | Threat model and CRY tests; define exact revocation scope | 004 |
| R-03 | Policy applied differently by agent, UI, sync and search | High | Critical | An endpoint or snippet fails the circle test | Single policy contract; end-to-end negative tests; deny-by-default | 005 |
| R-04 | An LLM invents or turns an extraction into a fact | High | High | Deadlines/participants without source | Per-field provenance, deterministic validation, review of critical fields | 008 |
| R-05 | Prompt injection triggers an external action | Med-high | Critical | Tool call derived from unverified email/PDF | Data/instruction separation, minimal scopes, approval, red-team, no direct model execution | 005, 008 |
| R-06 | Retry duplicates a send or remote mutation | Medium | High | Duplicates after timeout | Idempotency keys, outbox, `outcome_unknown`, reconciliation | 007 |
| R-07 | AI provider changes quality or behaviour | High | Med-high | Eval regression or more abstentions | Versioning, periodic evals, fallback, multi-provider routing | 008, 009 |
| R-08 | Fragmented on-device support | High | High | Feature available on few devices | Real device matrix, adapters, declared fallback | 009, 012 |
| R-09 | E2EE prevents planned query/AI capabilities | High | High | Cloud requests need unauthorized plaintext | Data-flow mapping per task; minimization; specific consent | 004, 011 |
| R-10 | MVP integrates every module before validating foundations | High | High | Many incomplete modules, no reliable end-to-end | One real vertical slice; target architecture kept in design | 001 |
| R-11 | Telemetry contains personal data | Medium | High | Prompts or titles in logs/traces | Redaction by default, log tests, access control, short retention | 010 |
| R-12 | Legal/consent risks underestimated in pilots | Medium | High | Minors'/health data shared without process | Pilot protocol, minimization, notices, professional review when needed | — |
| R-13 | Vision SLOs/thresholds unrealistic or unmeasured | Med-high | Med-high | No baseline or metric definition | Formal measurement protocol; recalibrate before claiming support | — |
| R-14 | The graph becomes an unevolvable universal schema | Medium | High | Many generic types, special-case queries | Bounded contexts, explicit types, versioned schema, real use cases | 001, 002 |
| R-15 | Unofficial integrations or over-broad scopes | Medium | High | Tokens with unnecessary permissions; fragile APIs | Official channels, minimal scopes, sandbox, expiry, terms review | — |

## Real data gate (report §11.1)

Move from synthetic or anonymised data, to the founder's own real data, to a limited circle of informed participants.
Each extension to minors, health, finance or other high-impact data needs a targeted review. This repository is an
engineering specification, not legal advice.
