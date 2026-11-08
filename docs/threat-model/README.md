# Threat model (baseline, WP0)

Status: **baseline v0.1** (specification, not implementation). It states what must be defended, which control each
threat requires, and which test or experiment will verify it. It does **not** claim any mitigation exists: every row is
`open` until its experiment produces evidence. Open decisions (report §14.2) stay open: controls are written as
*requirement + verifier + deciding ADR*. ADR-004 and ADR-005 depend on this document; Gate G0 requires it.

Sources: report §4.5, §4.12, §5, §8.2, §11. References: OWASP ASVS 5.0, OWASP Top 10 for LLM Applications,
NIST AI RMF and AI 600-1, NIST Privacy Framework. They guide the model; they are not certifications.

## Scope and assets

| Asset | Examples |
|---|---|
| Personal and shared-circle data | Persons, tasks, events, documents, sources |
| Keys and secrets | Circle/group keys, device keys, recovery material |
| Device state | Local store, indexes, caches, temp files, notifications |
| Connector credentials | OAuth tokens and their scopes |
| Audit trail | Policy decisions, approvals, executions |
| Derived data | Embeddings, snippets, summaries, search indexes |

## Actors and trust boundaries

Actors: circle member · former member · lost/revoked device · backend operator · external AI provider · connector
(third-party service) · **hostile imported content** (email, PDF, invitation, web page) · the model itself.

Boundaries, each crossing needs authentication, a policy decision and a declared plaintext status (see the map below):

```text
device ──(sync/relay)── backend API ── orchestrator/workflow ── worker (OCR/STT/AI) ── external provider
   │                          │                  │                       │
 local store          object store / DB     logs, traces, audit      backups
```

Rule that holds regardless of the open ADRs: **the model is not a trust boundary and never holds authority.** Imported
content is data, never instructions; mutating actions are decided by policy outside the model (§4.12).

## Threats

Columns: *Control required* is what must be true; *Verifier* is the test/experiment that will show it; *Status*
`open` = no evidence yet. Experiments live in [`docs/experiments`](../experiments/TEMPLATE.md) (cards: DAT-01 now,
CRY-01, POL-01, WFE-01, CL-01 as they land).

| # | Threat | Control required | Verifier | Risk | ADR | Status |
|---|---|---|---|---|---|---|
| T-01 | Compromised account; stolen or unlocked device | Device-bound keys; session and device revocation; local store protected at rest; step-up for high-risk actions | CRY-01 (device add/remove), security suite AR-15 | R-02 | 004 | open |
| T-02 | Device lost or revoked but still offline | Revocation scope stated precisely: future access, historical data, copies already downloaded, residual metadata are four separate answers; rekey on removal | CRY-01 case "offline at revocation"; DAT-01 S-09 | R-02 | 004 | open |
| T-03 | Malicious member, or member leaving the circle | Removal rotates keys; history policy explicit (what a leaver keeps); membership change is audited | CRY-01 remove-member + rotation; POL-01 membership cases | R-02 | 004, 005 | open |
| T-04 | Cross-circle access through cache, search, inference or notification | Authorisation filter **before** results/snippets/embeddings are produced; notifications and caches carry no unauthorised content; one policy contract for all surfaces | Security suite AR-04 using DAT-01 canaries (`CANARY-*` must never appear); cross-surface policy tests AR-14 | R-03 | 005, 011 | open |
| T-05 | Stolen or over-scoped OAuth token | Minimal scopes, expiry, rotation, vault storage, first connector read-only with test accounts | Connector contract tests (WP7); scope review per connector | R-15 | — | open |
| T-06 | Prompt injection in email, PDF, invitations, web pages | Imported content is untrusted data; model receives minimised authorised context; no tool call derives from unverified content without approval | Red-team suite, "zero unauthorised actions in the critical suite" (§8.3) | R-05 | 005, 008 | open |
| T-07 | Tool poisoning / deceptive MCP tool descriptions | Allowlisted tools with typed schemas, pinned descriptions, minimal scopes, timeouts | Tool-contract tests; description-change detection | R-05 | 008 | open |
| T-08 | Invalid, invented or ambiguous model output | Schema validation, deterministic checks, per-field provenance (AR-06, enforced in `regia-domain`), review of critical fields, appropriate abstention | AI eval harness per task family (WP4); provenance contract suite | R-04 | 008 | open |
| T-09 | Duplicate execution after timeout | Idempotency key + execution id per side effect; ambiguous outcome is `outcome_unknown` and reconciled before any retry | WFE-01 fault injection (target: 0 duplicate side effects) | R-06 | 007 | open |
| T-10 | Provider unavailable, unexpected retention, wrong routing | Routing by label (`secret` never to cloud in clear), declared retention, fallback that is non-mutating, no hidden failure | Adapter contract tests; routing negative tests | R-07 | 008, 009 | open |
| T-11 | Backups and logs holding deleted or unencrypted data | Redaction by default, no content in telemetry attributes, retention bounded, deletion reaches backups or is documented as not reaching them | Log-leak tests AR-12; restore-and-delete exercise (§8.2 gate 5) | R-11 | 010 | open |
| T-12 | Legitimate user with excessive privileges | Least privilege, roles from the policy taxonomy, privileged access audited and itself auditable | POL-01 negative matrix; audit review | R-03 | 005 | open |
| T-13 | Data loss, corruption or partial restore | Verified export/import round-trip, crash-safe writes, rebuildable indexes, tested restore | DAT-01 S-07/S-11; export round-trip AR-11 | R-01 | 002, 003 | open |

## Plaintext map (report §4.5)

For each location: **who** can read plaintext, **why**, **for how long**, under **which consent**, and which
`ServerVisibility` value governs it (`core/domain/src/visibility.rs`). Cells marked **TBD** are decisions, not
omissions: they depend on ADR-004 and on the owner's answers ([#15](https://github.com/AlessioBrillo/regia/issues/15)).
"Server without access to content" must be true for the **whole declared path**, not only for a database table.

| Location | Plaintext readable by | Why | Duration | `server_readable` | `end_to_end_encrypted` | `transient_worker` |
|---|---|---|---|---|---|---|
| Device storage | Device user, app | Local-first operation | Until deleted | n/a | At rest per OS + app key (TBD, ADR-004) | n/a |
| Process RAM | App process | Rendering, editing | Process lifetime | n/a | Decrypted while in use | n/a |
| Sync service / relay | Operator | Relay and merge | While stored/in transit | Yes | **No** — ciphertext only; metadata still visible (TBD) | No |
| Orchestrator / workflow | Operator code path | Run workflows | Task duration | Yes | Only if a task is explicitly authorised for plaintext | Yes, scoped to task |
| Logs / traces / audit | Authorised operator | Diagnosis, accountability | Short, bounded (TBD) | **Never content** | **Never content** | **Never content** |
| AI provider (cloud) | Provider | Inference | Provider-declared retention (TBD) | Only with specific consent | Only with specific consent | Only with specific consent |
| OCR / STT | Worker (local or remote) | Extraction | Task duration | Yes | Only local, or with consent | Yes |
| Object store | Operator | Files and media | Until deleted | Yes | Ciphertext only | No |
| Backups | Operator | Recovery | Retention window (TBD) | Yes | Ciphertext only | No |
| Indexes / embeddings | Index owner | Search, retrieval | Until rebuilt/deleted | Filtered by policy | Local-only index, or TBD | No |
| Caches / notifications / temp files | OS, app | Latency, alerts | Short | Minimal, no unauthorised content | Minimal, no unauthorised content | Minimal |

Binding now, independent of open ADRs: `Confidentiality::Secret` is "never sent to a cloud model or relay in the
clear" (`core/domain/src/label.rs`); an unknown label or visibility is rejected, never defaulted.

## Agent controls (binding now, report §4.12)

Imported content is untrusted data, never instructions · the model gets only authorised, minimised context · tools have
typed schemas, minimal scopes, allowlists and timeouts · mutating actions pass policy enforcement outside the model ·
proposed ≠ executed · approval binds to a precise summary (what, to whom, on which data, with which effect) and cannot be
reused for another version of the plan or recipient · every side effect has an idempotency key and execution id ·
ambiguous outcomes are reconciled before retrying · the "guardian" may flag or deny but holds no write credentials and
not the last word on policy · policy decisions and AI results stay distinguishable in the audit · policy errors are
`deny`, never `allow`.

## Pending owner decisions

Proposed defaults are deny-by-default and **not decided**; they become decisions only through
[#15](https://github.com/AlessioBrillo/regia/issues/15) and the ADRs.

| Question (report §14.3) | Proposed default | Decides |
|---|---|---|
| Q3 — E2EE for all shared objects or a labelled subset | Subset first (`restricted`, `secret`); revisit after CRY-01 | ADR-004 |
| Q4 — Data inaccessible to the operator; server-side workflows | Everything labelled `secret`; workflows on it run on-device | ADR-004, 008 |
| Q5 — Revoked member returns online with modified data | Rejected and quarantined for owner review, never merged silently | ADR-003, 004 |
| Q8 — Who can read admin logs | Founder only, access audited, no content in logs | ADR-010 |

## Out of scope for this baseline

Legal/consent assessment for minors, health or finance data (R-12), formal verification, and the independent security
review required before widening trials (AR-15). This repository is an engineering specification, not legal advice.
