# Security Policy

## Reporting a vulnerability

Use GitHub private vulnerability reporting: **Security tab → Report a vulnerability**. Do not open public issues for
security problems and do not include real personal data in reports.

Targets (best effort while the project is pre-release): acknowledgement within 3 business days, triage within 7 days,
coordinated disclosure after a fix or 90 days, whichever comes first.

## Scope

In scope: this repository's code, build and CI configuration, and the documented security properties (policy
enforcement, circle isolation, provenance, audit). Out of scope: third-party services and dependencies (report
upstream), social engineering, denial of service by volume.

## Supported versions

No release exists yet. Only `main` is supported.

## Principles

- No custom cryptography: only maintained, reviewed libraries.
- Policy is deny-by-default and evaluated outside any model.
- No personal data in logs, traces or fixtures by default.
- Claims such as "revocation" or "E2EE" are bounded by the [threat model](docs/threat-model/README.md); copies already
  seen by a member cannot be recalled.
