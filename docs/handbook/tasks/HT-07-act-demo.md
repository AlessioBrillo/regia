# HT-07 — Demo end-to-end: azione governata e audit

| Campo | Valore |
|---|---|
| ID | HT-07 |
| Area | development e verifica della slice |
| Tempo massimo | 2 h |
| Stato | pronta |
| Issue | da aprire con template `Human task` |

## Perché serve una persona

Un agente può eseguire i comandi e produrre il RAW, ma serve una persona a decidere se la proposta d'azione,
l'approvazione richiesta e il record di audit sono comprensibili: si capisce cosa è stato autorizzato e da dove viene,
senza che il record contenga il testo? Sono giudizi di chiarezza, non decisioni architetturali.

## Obiettivo

Eseguire una cattura sintetica, trasformarla in una proposta d'azione, negarla o approvarla esplicitamente e ritrovare
il corrispondente record di audit, verificando che la catena sia comprensibile e che nessun contenuto entri nell'audit.

Questa demo **non è un prodotto**: è un eseguibile del dominio con dati sintetici. Non accetta alcun ADR, non decide
storage, sync, policy engine, chiavi o runtime di workflow. La decisione di policy è fornita da chi esegue il comando;
il modello non autorizza nulla. Il *reminder* della catena G2 è **differito** e non è rappresentato in questo
incremento. Gli stati AR e gli ADR restano invariati: una demo non è evidenza di verifica.

## Input e prerequisiti

- Card e issue collegate
- HT-01 completata (ambiente verificato) e HT-06 completata (store `target/demo-store.jsonl` già presente)
- `regia-domain` con feature `synthetic`; solo il testo sintetico della card, nessun dato personale

## Procedura

1. Se lo store non esiste, esegui `just demo` e conserva l'id stampato da `capture`.
2. Esegui `act` senza `--policy`: la decisione deve essere `deny`, lo stato `denied` e il risultato dell'audit
   `not_attempted`. Nessuna esecuzione deve comparire nell'output.
3. Esegui `act` con `--policy allow` e rispondi `n`: nessuna esecuzione, audit con decisione negata.
4. Esegui `act` con `--policy allow` e rispondi `y`: la catena deve arrivare a `completed` e l'audit deve riportare
   `allow` / `succeeded`.
5. Apri `target/demo-audit.jsonl`: verifica che contenga solo identificatori, tempi, decisione, versione di policy,
   risultato e riferimenti a sorgenti. Il campo `actor` è il principal sintetico dichiarato, stampato anche
   nell'output come `principal:`. Non deve contenere il testo catturato, né `CANARY-`.
6. Con `--policy allow` e hash di default compare un `warning`: i valori demo rendono i controlli di binding
   vacui. Se vuoi una verifica significativa, passa `--policy-hash` e `--proposal-hash` espliciti.
7. Verifica che l'output riporti `sources`, `refused lines` e `withheld` separati per etichetta e cerchio.
8. I casi negativi (approvazione stale, scaduta, cerchio diverso, contesto di policy cambiato) sono coperti dai test
   automatici `cargo test -p regia-domain --test slice` e `--test workflow`: non serve ripeterli a mano.
9. Conserva il RAW in locale, fuori dal repo. Non copiare input reali.

## Comandi

```sh
# Reproduce lo store sintetico, se necessario:
just demo

# Trace del flusso con default deny (fail closed):
cargo run -p regia-domain --features synthetic --example act -- \
  --store target/demo-store.jsonl \
  --circle 01a21ee0-d408-7000-8000-000000000001 \
  --audit-store target/demo-audit.jsonl --q "anna"

# Catena completa, con approvazione umana (rispondi y o n):
cargo run -p regia-domain --features synthetic --example act -- \
  --store target/demo-store.jsonl \
  --circle 01a21ee0-d408-7000-8000-000000000001 \
  --audit-store target/demo-audit.jsonl --q "anna" --policy allow

# In alternativa a --q, punta a un id preciso stampato da capture:
cargo run -p regia-domain --features synthetic --example act -- \
  --store target/demo-store.jsonl \
  --circle 01a21ee0-d408-7000-8000-000000000001 \
  --audit-store target/demo-audit.jsonl --object <id-stampato-da-capture> --policy allow

# Test dei casi negativi:
cargo test -p regia-domain --features synthetic --test slice
cargo test -p regia-domain --test workflow
```

`--audit-store` è un file JSONL in append, come lo store della demo: eseguire più volte aggiunge righe. Entrambi i file
stanno sotto `target/`, che è gitignored, e devono essere percorsi diversi. `--q` deve corrispondere a **una sola**
attività visibile: se ne trova diverse, il comando rifiuta e bisogna usare `--object` con l'id stampato da `capture`.
`--principal` (richiedente, default sintetico `01a21ee0-d408-7000-8000-000000000002`) e `--approver` (approvatore,
default `01a21ee0-d408-7000-8000-000000000003`) sono identità distinte; l'audit riporta il richiedente come `actor`,
mentre l'approvatore compare nella traccia dell'evento. `--risk-class` e `--reversibility` sono dichiarazioni del
chiamante, non decisioni di policy. `--policy-hash`, `--proposal-hash`, `--idempotency-key`,
`--policy-version` e `--action-type` hanno valori di default dichiaratamente demo e possono essere sostituiti; non
sono fingerprint reali.

## Evidenze da conservare

- Output grezzo dei tre comandi, con data, OS e versione di `rustc`
- Conferma che il default è `deny` e che `n` non produce esecuzione
- RAW della risposta `y`, con gli stati attraversati fino a `completed`
- Contenuto di `target/demo-audit.jsonl`: campi presenti, assenza di testo catturato e di `CANARY-`
- Conteggio `sources`/`withheld`/`refused lines`, i campi `principal`/`approver` e risultati dei due comandi di test

## Criteri di accettazione

- [ ] Default `deny`: stato `denied`, audit `not_attempted`, nessuna esecuzione
- [ ] `--policy allow` + `y`: stato `completed`, audit `allow` / `succeeded`, sorgente citata
- [ ] `--policy allow` + `n`: nessuna esecuzione, audit negato
- [ ] L'audit non contiene payload, prompt, testo catturato o `CANARY-`
- [ ] Righe malformate e oggetti trattenuti contati, non silenziati; selezione ambigua rifiutata
- [ ] Test dei casi negativi eseguiti e verdi
- [ ] Nessun dato personale nei file prodotti
- [ ] Nessun ADR accettato, nessuna decisione aperta ratificata, stati AR invariati

## Dove finisce il risultato

Commento sull'issue con esito e blocchi; RAW in locale, non versionato. Se trovi un'ambiguità nel testo di questa card,
apri una PR con la correzione oppure commenta con `needs-owner`.

## Se ti blocchi

Commenta con `needs-owner` allegando il comando e l'errore. Non usare dati reali per aggirare il blocco e non
modificare una soglia per far passare la verifica.
