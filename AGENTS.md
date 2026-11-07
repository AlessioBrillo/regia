# AGENTS.md

Instructions for AI coding agents (and humans) working in this repository.

## What this is

Regìa: local-first, governed AI operations layer. Read [`README.md`](README.md) for the binding principles and
[`docs/vision/technical-report-v1.0.it.md`](docs/vision/technical-report-v1.0.it.md) (Italian) for the source report.
Section numbers (§) below refer to that report.

## Rules

- Do not ratify an open decision in code. Sync engine, MLS, Cedar vs OPA, Temporal, client framework and AI providers
  are open (§14.2); they change only through an accepted ADR backed by an experiment (§7.3).
- Never implement cryptographic primitives. Use maintained, reviewed libraries (§4.5).
- Policy errors are `deny`. Never map an error to `allow`.
- No personal data in logs, traces, fixtures or prompts. Datasets are synthetic.
- Every extracted fact needs provenance or an explicit reason (AR-06).
- Keep changes small. One logical change per commit, Conventional Commits, `Signed-off-by` (`git commit -s`).
- Commits are GPG-signed by the machine's default git config. Never override `user.*`, `commit.gpgsign` or
  `gpg.*` locally and never use `--no-verify` / `--no-gpg-sign`.

## CI gate

GitHub Actions is **not** the gate. The gate is `.woodpecker/ci.yml` (fmt, clippy, tests, docs build, `cargo deny`,
gitleaks, markdownlint, typos).

- **Local, no server:** `ci-local` from the repo root (Docker running). Exit 0 = green. It runs on a copy of the
  working tree; never run `woodpecker-cli exec` directly on the checkout.
- **Server:** push the branch and watch the `ci/woodpecker/pr/ci` check (`gh pr checks <n> --watch`).
- Before declaring a task done: gate green on the final tree. If it fails, report the failing step and log excerpt.

## Layout

`core/domain` (crate `regia-domain`) · `docs/adr` · `docs/requirements` · `docs/risk-register.md` ·
`docs/threat-model` · `docs/experiments`. `apps/`, `backend/`, `schemas/`, `evals/` do not exist yet by design.
