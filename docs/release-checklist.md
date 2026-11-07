# Experimental release checklist

Source: technical report v1.0, Appendix B. Non-negotiable gates before real personal data of several people (§8.2):
no open critical vulnerability in the reviewed perimeter · no mutating action without a policy decision outside the
model · no unauthorized content retrievable via query, search, snippet, prompt, notification, export or sync in the
adversarial tests · no duplicated side effects in the defined retry cases · export, restore and migration exercised on
controlled non-fictitious data · comprehensible revocation and incident procedures · participants told what is
processed and who accesses it.

- [ ] Requirements and use cases of the release traced
- [ ] Threat model updated for new features
- [ ] Policy and negative tests pass
- [ ] Migration, restore and export verified
- [ ] Offline, retry and duplicate-side-effect tests pass
- [ ] AI datasets/evals run and regressions reviewed
- [ ] Model, prompt, schema and policy versions recorded
- [ ] OAuth scopes and secrets verified
- [ ] Audit and telemetry free of unnecessary personal payload
- [ ] Disclosures, permissions and consent UX verified
- [ ] Known limitations and fallbacks visible
- [ ] Runbook and rollback ready
- [ ] Participants informed; exit path available
- [ ] Residual risks accepted by the owner for this phase
