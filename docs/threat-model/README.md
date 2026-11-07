# Threat model (baseline, WP0)

Status: **skeleton**. It enumerates what must be covered (report §4.12, §4.5); it does not yet claim any mitigation is
implemented. Gate G0 requires it to be completed; ADR-004 and ADR-005 depend on it.

## Scope and assets

Personal and shared-circle data; keys and secrets; device state; connector credentials (OAuth); the audit trail;
derived data (embeddings, snippets, caches, temporary files, notifications).

## Threats to cover

| # | Threat | Related risk | Status |
|---|---|---|---|
| T-01 | Compromised account; stolen or available device | R-02 | open |
| T-02 | Device lost or revoked but still offline | R-02 | open |
| T-03 | Malicious member, or a member leaving the circle | R-02 | open |
| T-04 | Cross-circle access via cache, search, inference or notification | R-03 | open |
| T-05 | Stolen or over-scoped OAuth token | R-15 | open |
| T-06 | Prompt injection in email, PDF, invitations, web pages | R-05 | open |
| T-07 | Tool poisoning / deceptive MCP tool descriptions | R-05 | open |
| T-08 | Invalid, invented or ambiguous model output | R-04 | open |
| T-09 | Duplicate execution after timeout | R-06 | open |
| T-10 | Provider unavailable, unexpected retention or wrong routing | R-07 | open |
| T-11 | Backups and logs containing deleted or unencrypted data | R-11 | open |
| T-12 | Legitimate user with excessive privileges | R-03 | open |
| T-13 | Data loss, corruption or partial restore | R-01 | open |

## Plaintext map (to fill, report §4.5)

For each location state *who* can read plaintext, *why*, *for how long*, and under which consent:
device storage · process RAM · sync service · orchestrator · logs · AI provider · OCR/STT · object store · backups.

## Agent controls that are binding now (report §4.12)

Imported content is untrusted data, never instructions · the model gets only authorised, minimised context · tools have
typed schemas, minimal scopes, allowlists and timeouts · mutating actions pass policy enforcement outside the model ·
proposed ≠ executed · approval binds to a precise summary (what, to whom, on which data, with which effect) and cannot be
reused for another version of the plan or recipient · every side effect has an idempotency key and execution id ·
ambiguous outcomes are reconciled before retrying · the "guardian" may flag or deny but holds no write credentials and
not the last word on policy.

## References

OWASP ASVS 5.0 · OWASP Top 10 for LLM Applications · NIST AI RMF and AI 600-1 · NIST Privacy Framework. These guide
the model; they are not certifications.
