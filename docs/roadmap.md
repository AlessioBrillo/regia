# Roadmap

Defined by technical outcomes and gates, not promised dates (report §13). A gate is a stop condition.

## Gates

| Gate | Output | Do not proceed if |
|---|---|---|
| **G0 — Verifiable baseline** *(closed 2026-11-08)* | Traced technical requirements, initial threat model, core entity schema, benchmark plan, preliminary policies, open ADRs | Source of truth, data classification, circles or risk scenarios are ambiguous |
| **G1 — Compared foundations** *(current)* | Decisions backed by tests on storage/sync, policy engine, key management, client strategy, workflow runtime | The system cannot explain who sees what, how a workflow resumes and how duplication is avoided |
| **G2 — Integrated vertical slice** | Capture → provenance → review → object → cited query → action proposal → approval → reminder → audit, with offline and circles | Critical bypasses, silent loss, false success claims, or no tested recovery remain |
| **G3 — Restricted pilot** | Controlled release on declared devices and scenarios, support and exit procedure | Participants do not understand data/access/limits, or revocation and recovery are unreliable |
| **G4 — Progressive extension** | Further modules and connectors on validated platform capabilities | An extension bypasses platform controls or lacks its own risk test |

## Work packages (report §7.1)

`WP0` requirements and measures → `WP1` domain and provenance (in parallel with spikes `WP2` offline/sync, `WP3`
policy and membership, `WP4` AI gateway/evaluation) → `WP5` durable workflows → `WP6` media/capture → `WP7` vertical
slice → `WP8` hardening and real trials. Do not consolidate the full backend before the sync, policy and workflow
spikes. The first real integration must be read-only or low-risk, with test accounts and minimal scopes.

## G0 checklist

- [x] Requirements AR-01..AR-15 each with verification method and owner
- [x] Threat model completed (plaintext map, T-01..T-13): baseline; open cells are decisions for ADR-004/005
- [x] Core entity schema v0 and contract suite (`core/domain`, logical schema, not a storage decision)
- [x] Benchmark plan DAT-01 and experiment cards for CL-01, CRY-01, policy and workflow spikes (plans; none executed)
- [x] Answers to the open questions (above)
- [x] ADR-001..012 each either accepted with evidence or explicitly scheduled ([ADR index](adr/README.md))

## G1 checklist

- [x] Policy engine: POL-01 executed, ADR-005 accepted as a Cedar trial (2026-11-12)
- [ ] Storage and sync: DAT-01 executed → ADR-002, ADR-003 (then ADR-001)
- [ ] Key management: CRY-01 → ADR-004
- [ ] Client strategy: CL-01 → ADR-012
- [ ] Workflow runtime: WFE-01 → ADR-007
- [ ] AI gateway and runtimes: AI-01 → ADR-008, ADR-009
- [ ] Backend shape: ADR-006, after DAT-01, POL-01 and WFE-01

## Owner answers to report §14.3 (2026-11-08)

Recorded in detail, with the experiment that verifies each, in
[`docs/threat-model/README.md`](threat-model/README.md#owner-decisions-2026-11-08-15).

1. Devices: Windows 11 desktop, a recent Android phone, a recent iPhone with a Mac (Xcode), the Arch Linux box
   ([CL-01](experiments/CL-01-client-comparison.md)).
2. Identity: passkey plus email (recovery, notifications); test accounts.
3. E2EE: mandatory first for the sensitive subset.
4. Hidden from the operator: sensitive content and private keys; routing metadata is a residual risk.
5. Revoked member returning online: quarantine on device plus admin review, never silent loss.
6. First-slice integrations: none, synthetic data only.
7. Scale: 1 person, 2-5 devices.
8. Backend: self-hosted on the Arch box, logs readable only by the owner.
9. Excluded from the pilot: health, minors, finance, credentials, ID documents.
10. Targets: local write p95 < 50 ms, convergence p95 < 5 s with 5 clients, 0 silent losses.

## Owner decision on scope (2026-11-15)

The vision is wider than families: one capture surface (voice or text) for work, home, notes, reminders and
commitments, filed automatically into well-separated areas, retrievable visually and by asking questions like to an
expert secretary. It should later replace agile ceremonies in structured organisations. Placement: personal and family
use stays the pilot (G3); organisations (AR-17) and agile as derived views (AR-18) belong to **G4**, and must reuse
the same policy engine and source of truth. AR-16 is the G2 slice target. Nothing in the G3 pilot perimeter changes.

## Owner decisions (2026-11-19)

- **Area is not a circle.** An *area* (work, home, project…) is "where it is filed"; a *circle* is "who can see it".
  Independent dimensions. One primary area per object, proposed by the model with evidence (`Assertion::Proposed`) and
  confirmed by the user; the list of areas is user-defined with a suggested starter set. Scope of AI-01 increment 2,
  not of increment 1 (its rules are frozen).
- **Voice** gets its own card, VOX-01, opened right after AI-01 increment 1 closes; the text transcript stays the
  auditable intermediate.
- **AI-01 runs:** R3 (Ollama) on the Arch box; R4 (Mistral EU) with synthetic input only, key from the environment.
  The router still blocks canary, `restricted` and `secret` cases from any cloud runtime.

## Out of the first vertical slice (not out of the vision)

Payments, open banking, agentic phone calls, public-administration automation, functional health assistance, advanced
ambient features, multiple specialist agents, A2A between agents, semantic search at scale, and all external
integrations not needed for the proof.
