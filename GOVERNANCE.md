# Governance

Regìa is currently led by its founder (benevolent-dictator model) with a path to a maintainer group.

## Roles (report §10.1)

| Area | Accountable |
|---|---|
| Vision and flow priorities | Product / Experiment Owner |
| Architecture and ADRs | Principal Engineer |
| Data and sync | Domain/sync tech lead |
| Security and privacy | Security owner (external reviewer encouraged) |
| AI and evaluation | AI engineer |
| Workflow and connectors | Backend engineer |
| Mobile and accessibility | Client engineer |

One person may hold several roles; the responsibilities stay distinct so urgency does not replace technical
evaluation. Project management ensures every work package has objective, output, acceptance criteria, owner and
dependencies. The principal engineer ensures coherence, evidence for choices and quality of contracts.

## Decisions

- Significant technical decisions are ADRs (`proposed → accepted | rejected | superseded`) with alternatives,
  evidence, consequences, rollback and reopening conditions. Template: [`docs/adr/0000-template.md`](docs/adr/0000-template.md).
- Experiments produce a reproducible artifact and a card ([template](docs/experiments/TEMPLATE.md)).
- Gates G0–G4 ([roadmap](docs/roadmap.md)) are explicit stop conditions; security gates are not averaged away.

## Becoming a maintainer

Sustained, high-quality contributions (code, ADR review, security findings) followed by an invitation from existing
maintainers. When a second maintainer joins: required approvals on `main` go to 1 with code-owner review, and the
repository may move to a GitHub organization.
