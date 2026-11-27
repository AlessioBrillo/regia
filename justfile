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

# Demo end-to-end su dati sintetici: cattura e query citata. Non è un prodotto e non ratifica ADR.
demo:
    cargo run -p regia-domain --features synthetic --example capture -- --text "Synthetic: call Anna tomorrow" --store target/demo-store.jsonl
    cargo run -p regia-domain --features synthetic --example query -- --store target/demo-store.jsonl --q "anna"
