# HT-02 — CL-01: inventario e baseline dei dispositivi disponibili

| Campo | Valore |
|---|---|
| ID | HT-02 |
| Area | prove dispositivi |
| Tempo massimo | 2 h |
| Stato | pronta |
| Issue | da aprire con template `Human task` |

## Perché serve una persona

Qualcuno deve eseguire materialmente i comandi su Windows 11 e Arch e dire quali casi sono davvero provabili, senza sostituire un dispositivo mancante con una simulazione.

## Obiettivo

Stabilire cosa si può provare oggi di CL-01 e documentare il gap dei dispositivi non ancora forniti.

## Input e prerequisiti

- HT-01 completata
- Windows 11 e Arch Linux disponibili
- Android, iPhone e Mac con Xcode **non** disponibili: il gap va riportato, non colmato
- [CL-01](../../experiments/CL-01-client-comparison.md) e il suo flow sotto test

## Procedura

1. Registra la versione esatta del sistema operativo e della toolchain per ogni macchina.
2. Genera il dataset sintetico di riferimento e controlla che non contenga dati personali.
3. Esegui l'harness DAT-01 sulle due macchine e conserva la riga grezza di ogni run.
4. Cerca un eseguibile nel repo che implementi il flow di CL-01: share, persist, proposal, confirm o cancel, sync, errore di rete. Oggi non esiste: se non lo trovi, scrivi "non eseguibile" e vai al punto 5. Non inventare un client.
5. Compila la tabella: dispositivo, caso, eseguibile, esito, nota. Per Android, iPhone e Mac scrivi "dispositivo non fornito".
6. Segnala all'owner la dipendenza hardware per sbloccare la parte mobile.

## Comandi

```sh
cargo run -p regia-domain --example dat01 -- 42 small > dataset.jsonl
cargo run -p regia-spike-dat01 --release --example bench -- 10
```

## Evidenze da conservare

- Raw output dei comandi (l'harness stampa una riga per run)
- `dataset.jsonl` generato, locale e sintetico, non versionato
- Tabella dispositivi/casi compilata

## Criteri di accettazione

- [ ] Tabella con almeno Windows 11 e Arch Linux
- [ ] Ogni caso CL-01 dichiarato eseguibile o non eseguibile, senza stime inventate
- [ ] Gap mobile esplicitato per Android, iPhone e Mac
- [ ] Nessun dato personale nei file conservati

## Dove finisce il risultato

`docs/experiments/cl-01/results-inventory-<data>.md` (crea la cartella se manca) e commento di stato sull'issue.

## Se ti blocchi

Commenta con `needs-owner`. Non chiudere CL-01: la card registra solo l'inventario, la decisione resta all'ADR-012.
