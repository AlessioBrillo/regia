# Contributing to Regìa

Thanks for helping. Regìa is at Gate G0 (see [roadmap](docs/roadmap.md)); the most valuable contributions right now
are experiments, ADR reviews, threat-model findings and test cases.

## Ground rules

- Be kind: see the [Code of Conduct](CODE_OF_CONDUCT.md).
- Security issues go through [SECURITY.md](SECURITY.md), never public issues.
- Use synthetic data only. Never attach real personal data to issues, PRs or fixtures.

## Workflow

Trunk-based: short-lived branches off `main`, small PRs, squash-merge. `main` is protected (PR required, linear
history, signed commits).

1. Open or pick an issue. Architecture-affecting changes start as an [ADR proposal](docs/adr/0000-template.md).
2. Branch: `feat/<topic>`, `fix/<topic>`, `docs/<topic>`.
3. Run the gate locally: `ci-local` (Docker) and, for Rust-only loops, `cargo test --workspace`.
4. Open a PR. The PR title is the squash commit message and must be a Conventional Commit
   (`feat(domain): …`, `fix:`, `docs:`, `ci:`, `build:`, `chore:`, `test:`, `refactor:`).

## Commits: signed and signed-off

- **Sign every commit** (GPG or SSH) with a key registered on your GitHub account so it shows as *Verified*:
  `git config commit.gpgsign true`.
- **Sign off** every commit under the [Developer Certificate of Origin](https://developercertificate.org/):
  `git commit -s`.
- Use your GitHub `noreply` address. Enable the guard: `git config core.hooksPath .githooks`.

## Definition of Done (report §9.2)

A change is done when it has: a traced requirement/acceptance criterion; updated data model and policy; unit,
integration and end-to-end tests as appropriate; specified retry/timeout/error cases; documented data handling;
redacted telemetry; audit for relevant actions; migration/export/deletion handled where applicable; accessibility and
copy checked; known limits and fallbacks stated; runbook if it needs an operated service; updated AI evals if it uses
a model.

## Decisions

Technology choices that the report leaves open are decided only by an accepted ADR with reproducible evidence
([experiment template](docs/experiments/TEMPLATE.md)). Scoring weights are frozen before results are seen.
