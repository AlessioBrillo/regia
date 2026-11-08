# Roadmap

Defined by technical outcomes and gates, not promised dates (report §13). A gate is a stop condition.

## Gates

| Gate | Output | Do not proceed if |
|---|---|---|
| **G0 — Verifiable baseline** *(current)* | Traced technical requirements, initial threat model, core entity schema, benchmark plan, preliminary policies, open ADRs | Source of truth, data classification, circles or risk scenarios are ambiguous |
| **G1 — Compared foundations** | Decisions backed by tests on storage/sync, policy engine, key management, client strategy, workflow runtime | The system cannot explain who sees what, how a workflow resumes and how duplication is avoided |
| **G2 — Integrated vertical slice** | Capture → provenance → review → object → cited query → action proposal → approval → reminder → audit, with offline and circles | Critical bypasses, silent loss, false success claims, or no tested recovery remain |
| **G3 — Restricted pilot** | Controlled release on declared devices and scenarios, support and exit procedure | Participants do not understand data/access/limits, or revocation and recovery are unreliable |
| **G4 — Progressive extension** | Further modules and connectors on validated platform capabilities | An extension bypasses platform controls or lacks its own risk test |

## Work packages (report §7.1)

`WP0` requirements and measures → `WP1` domain and provenance (in parallel with spikes `WP2` offline/sync, `WP3`
policy and membership, `WP4` AI gateway/evaluation) → `WP5` durable workflows → `WP6` media/capture → `WP7` vertical
slice → `WP8` hardening and real trials. Do not consolidate the full backend before the sync, policy and workflow
spikes. The first real integration must be read-only or low-risk, with test accounts and minimal scopes.

## G0 checklist

- [ ] Requirements AR-01..AR-15 each with verification method and owner
- [ ] Threat model completed (plaintext map, T-01..T-13)
- [x] Core entity schema v0 and contract suite (`core/domain`, logical schema, not a storage decision)
- [ ] Benchmark plan DAT-01 and experiment cards for CL-01, CRY-01, policy and workflow spikes
- [ ] Answers to the open questions below
- [ ] ADR-001..012 each either accepted with evidence or explicitly scheduled

## Open questions for the owner (report §14.3)

1. Which iOS/Android/desktop devices and OS versions are available for the first experiment?
2. Which account/identity model is realistic initially (email, passkey, login provider, recovery)?
3. Does E2EE apply to all shared objects or first to a subset with sensitive labels?
4. Which data must stay inaccessible even to the backend operator, and which workflows need server-side processing?
5. What is the expected behaviour when a revoked member returns online with previously modified data?
6. Which real systems (calendar, email, documents) are used in the first slice, and which have official API access?
7. How many participants and concurrent devices are expected in the initial test?
8. Where does the test backend run and who can access administrative logs?
9. Which data categories are excluded from the first pilot until specific checks are done?
10. Which latency/availability targets are decisive for the first use case, and how will they be measured?

## Out of the first vertical slice (not out of the vision)

Payments, open banking, agentic phone calls, public-administration automation, functional health assistance, advanced
ambient features, multiple specialist agents, A2A between agents, semantic search at scale, and all external
integrations not needed for the proof.
