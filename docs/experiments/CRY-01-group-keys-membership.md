# CRY-01 — Group keys and membership

Status: **plan, not executed.** Report §4.5, §5.6. Feeds ADR-004. Owner: TBD ([#17](https://github.com/AlessioBrillo/regia/issues/17)).

```text
ID / version / owner:            CRY-01 / v1 (plan) / TBD
Technical question:              Can circle membership changes (invite, add device, removal, offline revocation)
                                 be expressed with a maintained group-key scheme, and what exactly does each
                                 change protect: future access, historical data, copies already downloaded,
                                 residual metadata?
Linked vision requirement:       AR-05; R-02, R-09; workflow WF-06
Initial hypothesis/alternatives: A  MLS (RFC 9420) through a maintained, reviewed implementation
                                 B  Per-circle symmetric keys with an audited key-wrapping scheme
                                 C  Server-managed encryption for some labels only
                                 No alternative is preferred. No proprietary cryptography under any option.
Relevant threat model:           T-01, T-02, T-03, T-11 (docs/threat-model)
Reproducible setup:              Library name + exact version + commit, OS, devices (see #14), network simulator
Method and test inputs:          Cases K-01..K-12; DAT-01 dataset (secret/restricted objects) as payload
Metrics collected and units:     See "Metrics"
Raw results and repetitions:     Not executed. Minimum 10 runs per case
Failed cases / limits:           Not executed
Effect on requirements:          Not executed
Decision: adopt/trial/defer/reject:  Not executed
Linked ADR:                      ADR-004 (also informs ADR-003, ADR-011)
Conditions that would reopen:    Library vulnerability or abandonment; iOS/Android/server support gap; a case
                                 outside K-01..K-12 shows access after revocation that the ADR did not declare
```

## Cases

For **every** case, the result table records four separate answers: **future access**, **historical data access**,
**copies already downloaded**, **residual metadata**. A case without all four answers is not complete.

| ID | Case |
|---|---|
| K-01 | Create a circle |
| K-02 | Invite and accept (acceptance and device authentication are distinct, §5.6) |
| K-03 | Add a new device for an existing member |
| K-04 | Remove a member (online devices) |
| K-05 | Key rotation, scheduled and on-compromise |
| K-06 | Messages in flight during a membership change |
| K-07 | Persisted data created before and after the change |
| K-08 | **Revocation while a device is offline**, device returns |
| K-09 | Backup taken before and after removal |
| K-10 | Recovery after total device loss; restore from backup |
| K-11 | Member leaves voluntarily vs is removed |
| K-12 | Two membership changes racing |

## Metrics

| Metric | Unit |
|---|---|
| Plaintext reachable after change, per the four answers | enumerated objects / bytes |
| Time for a change to take effect on online / offline devices | s |
| Messages and bytes per membership change at group sizes 2, 5, 50 | count, bytes |
| Recovery success without server-side key escrow | pass/fail |
| Library maturity: releases, audits, open advisories, platform coverage | recorded facts |
| Integration effort and failure diagnosis | hours, injected faults diagnosed |

## Gate and scoring

1. **Gate first:** no access to data created after a removal; every residual access is documented and matches what the
   ADR declares; backup/recovery works without an undeclared escrow; no custom primitive.
2. Survivors scored with the frozen weights in [`TEMPLATE.md`](TEMPLATE.md).
3. Key backup and recovery are a **critical function**, tested as such, not a UX detail (§4.5).

## Out of scope

Policy evaluation (POL-01), data-model sync (DAT-01). Legal effect of "deleted" data in downloaded copies is stated
as a limit, not solved.
