# WFE-01 — Workflow runtime spike (state machine + outbox vs Temporal)

Status: **plan, not executed.** Report §4.7, §5.3. Feeds ADR-007. Owner: @AlessioBrillo ([#17](https://github.com/AlessioBrillo/regia/issues/17)).

Naming: the report calls this spike "WF-01", which collides with workflow WF-01 of §5.1 (capture → proposal). This
repository calls the spike **WFE-01** (workflow *engine*). ADR-007 still says "WF-01 (§4.7)"; it is clarified when that
ADR is next edited.

```text
ID / version / owner:            WFE-01 / v1 (plan) / @AlessioBrillo
Technical question:              Can a persistent state machine + outbox meet recovery, timer and human-wait
                                 requirements, or does durable execution (Temporal) remove enough risk to justify
                                 its operational and data-handling cost?
Linked vision requirement:       AR-07, AR-08; R-06; workflows WF-03, WF-05
Initial hypothesis/alternatives: A  Persistent state machine + transactional outbox + workers
                                 B  Temporal or an equivalent durable-execution runtime
                                 No alternative is preferred; Temporal is not introduced merely because an agent exists.
Relevant threat model:           T-08, T-09 (docs/threat-model)
Reproducible setup:              Runtime + version, OS, storage, simulated external service, fault injector
Method and test inputs:          The riskiest realistic workflow, below; cases W-01..W-10
Metrics collected and units:     See "Metrics"
Raw results and repetitions:     Not executed
Failed cases / limits:           Not executed
Effect on requirements:          Not executed
Decision: adopt/trial/defer/reject:  Not executed
Linked ADR:                      ADR-007
Conditions that would reopen:    Recovery or timer requirements grow beyond what was tested; sensitive payloads
                                 cannot be kept out of history; operating cost changes materially
```

## Workflow under test

`input → extraction → approval → simulated external side effect → timeout → retry → reconciliation`, following the
WF-03 state machine: `requested → … → policy_evaluated → approved → execution_started → succeeded | failed |
outcome_unknown → reconciled → completed | compensated`. The external system is a **simulator** that can accept then
time out, accept twice, fail permanently, or answer late. No real account is used.

## Cases

| ID | Case | Required result |
|---|---|---|
| W-01 | Process crash between each pair of steps | Resumes; no step lost |
| W-02 | Timeout after the simulator accepted the request | State `outcome_unknown`, reconciled before any retry |
| W-03 | Retry and replay with the same idempotency key | Exactly one side effect |
| W-04 | Double click / duplicate delivery | Exactly one execution |
| W-05 | Approval expires; payload or recipient changes after approval | Approval invalid; new approval required |
| W-06 | Permissions change or token revoked while waiting for approval | Re-evaluated; denied if no longer valid |
| W-07 | Human wait of hours/days; timer survives restart | Resumes at the right time |
| W-08 | Workflow definition changes while instances are in flight | Declared versioning behaviour |
| W-09 | Cancellation, and compensation of a reversible action | Declared as cancellable / compensable / irreversible |
| W-10 | Sensitive payload handling | Not copied into history/logs beyond what is declared |

## Metrics

| Metric | Unit |
|---|---|
| Duplicate side effects across all fault injections | count (must be 0) |
| Faults ending in a consistent, observable state | % (target 100%, §8.3) |
| Unauthorised executions | count (must be 0) |
| Recovery time after crash | s |
| Code and operational surface: LOC, services to run, on-call procedures | counts |
| Diagnosis: time to explain an injected failure from traces alone | min |
| Sensitive data present in workflow history/logs | objects found (target 0) |

## Gate and scoring

1. **Gate first:** zero duplicate side effects, zero unauthorised executions, 100% of injected faults in a coherent
   observable state, and external-side-effect state verified, not just the internal database (§8.3). Idempotency of
   external effects is required under either option.
2. Survivors scored with the frozen weights in [`TEMPLATE.md`](TEMPLATE.md).
3. Criteria to adopt Temporal (§4.7): recovery and timers clearly simpler to demonstrate, adequate human-wait and
   versioning support, sensitive payloads kept out of history, acceptable operations.

## Out of scope

Real connectors (WF-07), AI model behaviour (WP4), reminders as a product feature (WF-05) beyond the timer test.
