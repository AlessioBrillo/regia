# Architecture Decision Records

Template: [`0000-template.md`](0000-template.md). Status lifecycle: `proposed → accepted | rejected | superseded`.
An ADR is accepted only with reproducible evidence from an [experiment](../experiments/TEMPLATE.md).

| ADR | Decision | Status | Scheduled (evidence · gate) |
|---|---|---|---|
| [001](001-rust-core-perimeter.md) | Perimeter of the shared Rust core | proposed | DAT-01 + CL-01 · G1 |
| [002](002-data-model-source-of-truth.md) | Data model and source of truth | proposed | DAT-01 · G1 |
| [003](003-sync-engine-conflict-semantics.md) | Sync engine and conflict semantics | proposed | DAT-01 · G1 |
| [004](004-key-lifecycle-e2ee-mls.md) | Key lifecycle, E2EE and MLS | proposed | CRY-01 · G1 |
| [005](005-policy-engine.md) | Policy engine and label taxonomy | proposed | POL-01 · G1 |
| [006](006-backend-modular-monolith.md) | Modular monolith, storage, backend API | proposed | After DAT-01, POL-01, WFE-01 · end of G1 |
| [007](007-workflow-runtime.md) | State machine + outbox vs Temporal | proposed | WFE-01 · G1 |
| [008](008-ai-gateway-task-router.md) | AI gateway and task router | proposed | [AI-01](../experiments/AI-01-ai-gateway-runtimes.md) · G1 |
| [009](009-ai-providers-runtimes.md) | On-device and cloud AI runtimes | proposed | [AI-01](../experiments/AI-01-ai-gateway-runtimes.md) · G1 |
| [010](010-telemetry-and-audit.md) | Telemetry and audit | proposed | Vertical slice · G2 |
| [011](011-search-and-indexes.md) | Search and indexes | proposed | Vertical slice · G2 (semantic search is out of the slice) |
| [012](012-native-vs-cross-platform.md) | Native vs cross-platform clients | proposed | CL-01 · G1 |
