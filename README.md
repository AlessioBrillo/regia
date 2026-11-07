# Regìa

> A local-first, privacy-preserving, governed AI operations layer for families and small circles of people.

**Status: Gate G0 (verifiable baseline) in progress.** Nothing here is production software and no data-protection
guarantee is made yet. See the [roadmap](docs/roadmap.md).

Regìa combines things that are individually tractable but hard to integrate: personal data shared across circles,
offline operation, multi-device sync, confidentiality and revocation, multimodal capture, local and remote language
models, durable agent workflows, external connectors and an audit trail people can read.

## Binding principles

These are decided; everything else is a hypothesis to be proven by a reproducible experiment and recorded as an
[ADR](docs/adr/).

1. **The agent is not an authority.** The model proposes; deterministic code outside the model authorizes and executes.
2. **Provenance is first-class data.** Every extracted fact carries a source, or an explicit reason why it has none.
3. **Policy is deny-by-default** and evaluated outside the model. An error is never an `allow`.
4. **End-to-end architecture first.** The MVP is one real vertical slice of the platform, not a throwaway demo.
5. **Derived data has a lifecycle.** Embeddings, snippets, caches and temporary files follow the same policy as sources.
6. **Thresholds are targets, not claims** until measured with a documented protocol.

## Repository map

| Path | Purpose |
|---|---|
| [`core/domain`](core/domain) | `regia-domain`: typed IDs, confidentiality labels, server visibility, provenance |
| [`docs/adr`](docs/adr) | Architecture Decision Records (ADR-001..012, all `proposed`) |
| [`docs/requirements`](docs/requirements/architecture-requirements.md) | Architecture requirements AR-01..AR-15 |
| [`docs/risk-register.md`](docs/risk-register.md) | Risks R-01..R-15 |
| [`docs/threat-model`](docs/threat-model/README.md) | Threat model baseline |
| [`docs/experiments`](docs/experiments/TEMPLATE.md) | Experiment card template and scoring weights |
| [`docs/vision`](docs/vision/technical-report-v1.0.it.md) | Original technical report v1.0 (Italian) |

## Development

```sh
ci-local          # full gate (needs Docker); exit 0 = green
cargo test --workspace
git config core.hooksPath .githooks   # reject unsigned / non-noreply commits before push
```

All commits must be GPG-signed and carry a `Signed-off-by` line; see [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

Please report vulnerabilities privately; see [SECURITY.md](SECURITY.md).

## License

Apache-2.0, see [LICENSE](LICENSE).
