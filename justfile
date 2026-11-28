# Comandi ripetibili per chi lavora sul progetto. Richiede `just`.
# Su Windows 11 usare `ci-local.cmd` da PowerShell per il gate completo:
# lo script `gate` qui sotto usa `ci-local`, disponibile nelle shell Unix.

# Prepara il controllo degli hook e stampa le versioni degli strumenti.
setup:
    git config core.hooksPath .githooks
    rustc --version
    cargo --version
    docker version

# Loop Rust veloce, senza Docker.
test:
    cargo test --workspace --locked

# Stessi controlli Rust del gate, senza Docker.
rust-checks:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    cargo doc --workspace --no-deps --locked

# Il gate vero: richiede Docker attivo (vedi .woodpecker/ci.yml).
gate:
    ci-local

# Controlli sui documenti, come nella pipeline docs.
docs:
    npx --yes markdownlint-cli2@0.24.0 "**/*.md" "#docs/vision" "#target" "#node_modules"
    typos

# Cerchio sintetico usato sia in cattura sia in query: nessun dato reale, nessuna decisione su policy.
DEMO_CIRCLE := "01a21ee0-d408-7000-8000-000000000001"

# Demo end-to-end su dati sintetici: cattura e query citata con vista ristretta al cerchio.
# Non è un prodotto e non ratifica ADR.
demo:
    cargo run -p regia-domain --features synthetic --example capture -- --text "Synthetic: call Anna tomorrow" --store target/demo-store.jsonl --circle {{DEMO_CIRCLE}}
    cargo run -p regia-domain --features synthetic --example query -- --store target/demo-store.jsonl --circle {{DEMO_CIRCLE}} --q "anna"

# Review interattiva: proposta deterministica di area e conferma umana. Non è un modello e non ratifica ADR.
review store="target/demo-store.jsonl":
    cargo run -p regia-domain --features synthetic --example review -- --store {{store}} --areas core/domain/examples/demo-areas.json --circle {{DEMO_CIRCLE}}

# Query con area risolta: da eseguire dopo `just review`.
query:
    cargo run -p regia-domain --features synthetic --example query -- --store target/demo-store.jsonl --circle {{DEMO_CIRCLE}} --areas core/domain/examples/demo-areas.json --q "anna"

# Azione governata: proposta da un oggetto catturato, policy deny-by-default e audit content-free.
# Di default nega; per la catena completa esegui `cargo run` con --policy allow (vedi HT-07).
act:
    cargo run -p regia-domain --features synthetic --example act -- --store target/demo-store.jsonl --circle {{DEMO_CIRCLE}} --audit-store target/demo-audit.jsonl --q "anna"
