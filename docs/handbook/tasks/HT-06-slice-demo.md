# HT-06 — Demo end-to-end: cattura e query citata

| Campo | Valore |
|---|---|
| ID | HT-06 |
| Area | development e verifica della slice |
| Tempo massimo | 2 h |
| Stato | pronta |
| Issue | da aprire con template `Human task` |

## Perché serve una persona

Un agente può eseguire i comandi e produrre il RAW, ma serve una persona a decidere se la citazione stampata è
sufficiente per capire da dove viene un fatto e se l'area proposta è quella giusta. Sono giudizi di comprensibilità,
non decisioni architetturali.

## Obiettivo

Eseguire una cattura sintetica, assegnarle un'area con conferma umana e ritrovarla con la sua citazione, verificando
che il risultato sia comprensibile e che nessun dato reale entri nel flusso.

Questa demo **non è un prodotto**: è un eseguibile del dominio con dati sintetici. Non accetta alcun ADR, non decide
storage o sync (ADR-002/003 restano aperti) e non riguarda policy o chiavi. La proposta di area usa regole
deterministiche dichiarate, non un modello: AI-01 increment 2 resta aperto.

## Input e prerequisiti

- HT-01 completata (ambiente verificato)
- `regia-domain` con feature `synthetic`
- Nessun dato personale: usare solo il testo sintetico della card

## Procedura

1. Esegui `just demo`. I due comandi sono anche riportati sotto, se vuoi eseguirli uno alla volta.
2. Leggi l'output di `query`: per ogni risultato devono comparire `id`, `area: not filed yet`, il testo e almeno una
   riga `source:` con `locator` e `captured_at`.
3. Controlla il conteggio finale: `refused lines: 0` (una riga malformata va contata, non nascosta) e
   `withheld` separato per etichetta e cerchio. Gli oggetti trattenuti non devono mai mostrare contenuto.
4. Esegui `query` senza `--circle`: deve essere rifiutata (fail closed), non deve stampare alcun oggetto.
5. Prova una cattura con testo contenente `CANARY-`: deve essere rifiutata, senza scrivere righe.
6. Esegui `just review` (o il comando sotto): per ogni oggetto non ancora filato devono comparire l'area proposta e
   l'evidenza. Rispondi `y` solo se l'area è corretta; `n` deve lasciare l'oggetto invariato.
7. Esegui `just query`: l'oggetto confermato deve mostrare `area: <nome> (<id>) [confirmed]` insieme alla citazione.
8. Su uno store generato da `dat01`, verifica che l'output di `query` non contenga mai `CANARY-`.
9. Conserva il RAW in locale, fuori dal repo. Non copiare input reali.

## Comandi

```sh
just demo

# Passi singoli, se serve diagnosticare:
cargo run -p regia-domain --features synthetic --example capture -- --text "Synthetic: call Anna tomorrow" --store target/demo-store.jsonl --circle 01a21ee0-d408-7000-8000-000000000001
cargo run -p regia-domain --features synthetic --example query -- --store target/demo-store.jsonl --circle 01a21ee0-d408-7000-8000-000000000001 --q "anna"

# Il rifiuto del canary non scrive righe:
cargo run -p regia-domain --features synthetic --example capture -- --text "CANARY-1-2" --store target/demo-store.jsonl --circle 01a21ee0-d408-7000-8000-000000000001

# Nessun oggetto senza --circle (fail closed):
cargo run -p regia-domain --features synthetic --example query -- --store target/demo-store.jsonl --q "anna"

# Review interattiva e query con area risolta:
just review
just query
cargo run -p regia-domain --features synthetic --example review -- --store target/demo-store.jsonl --areas core/domain/examples/demo-areas.json --circle 01a21ee0-d408-7000-8000-000000000001
cargo run -p regia-domain --features synthetic --example query -- --store target/demo-store.jsonl --circle 01a21ee0-d408-7000-8000-000000000001 --areas core/domain/examples/demo-areas.json --q "anna"
```

`--store` è un file JSONL in append: eseguire la demo più volte aggiunge righe, non le sovrascrive. Il file sta sotto
`target/`, che è gitignored.

## Evidenze da conservare

- Output dei due comandi (RAW), con data, OS e versione di `rustc`
- Conteggio `matches`, `withheld` (per etichetta e cerchio) e `refused lines`
- Conferma che il tentativo `CANARY-` è stato rifiutato e che nessun `CANARY-` compare in output
- Conferma che la query senza `--circle` è rifiutata
- RAW della review (proposta + evidenza + risposta) e conteggio `confirmed`/`skipped`
- Eventuale output della query con un termine che non esiste: zero risultati

## Criteri di accettazione

- [ ] Un oggetto catturato e ritrovato dalla query
- [ ] Citazione con `source`, `locator` e `captured_at` presente nell'output
- [ ] `area: not filed yet` prima della review, poi `area: <nome> (<id>) [confirmed]`
- [ ] Ogni assegnazione ha un'evidenza: proposta con source della cattura, conferma con reason
- [ ] Righe malformate contate, non silenziate
- [ ] Oggetti fuori cerchio e `restricted`/`secret` contati in `withheld`, mai stampati
- [ ] Query senza `--circle` rifiutata; nessun `CANARY-` in output
- [ ] Input `CANARY-` rifiutato
- [ ] Nessun dato personale nei file prodotti
- [ ] Nessun ADR accettato e nessuna decisione aperta ratificata

## Dove finisce il risultato

Commento sull'issue con esito e blocchi; RAW in locale, non versionato. Se trovi un'ambiguità nel testo di questa card,
apri una PR con la correzione oppure commenta con `needs-owner`.

## Se ti blocchi

Commenta con `needs-owner` allegando il comando e l'errore. Non usare dati reali per aggirare il blocco e non
modificare una soglia per far passare la verifica.
