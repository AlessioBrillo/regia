# HT-04 — Human eval AI-01 su casi sintetici

| Campo | Valore |
|---|---|
| ID | HT-04 |
| Area | human eval |
| Tempo massimo | 2 h |
| Stato | pronta |
| Issue | da aprire con template `Human task` |

## Perché serve una persona

La qualità percepita di area, citazione e tono è un giudizio umano: un agente può produrre il punteggio, ma non può sostituire la tua valutazione su 20 casi.

## Obiettivo

Valutare 20 casi sintetici con la [rubrica](../human-eval-rubric.md) e produrre un conteggio per criterio, non una decisione architetturale.

## Input e prerequisiti

- HT-01 completata
- Golden set sintetico di AI-01 in `spikes/ai01`
- Rubrica letta prima di eseguire il comando

## Procedura

1. Esegui il runtime `r0` sul golden set, limitato a 20 casi, scrivendo il RAW in un file locale.
2. Prima di guardare il punteggio stampato, compila la rubrica per i primi 5 casi.
3. Valuta i restanti 15 casi con la stessa rubrica.
4. Applica il gate: canary, dati reali o azioni autorizzate senza approvazione rendono il caso FAIL indipendentemente dal punteggio.
5. Conta i risultati per criterio (`area`, `citation`, `intent`, `tone`) e lista i casi ambigui.
6. Conserva il `raw.jsonl` in locale: non versionarlo e non copiarvi input reali.

## Comandi

```sh
cargo run -p regia-spike-ai01 --release --example run -- --runtime r0 --limit 20 --out raw.jsonl
```

## Evidenze da conservare

- `raw.jsonl` locale con una riga per caso
- Conteggi per criterio e casi con gate FAIL
- Versione del runtime e data della valutazione

## Criteri di accettazione

- [ ] 20 casi valutati con la rubrica
- [ ] Primi 5 casi giudicati prima di leggere il punteggio dell'harness
- [ ] Gate applicato e casi FAIL elencati
- [ ] Nessun dato personale nel RAW

## Dove finisce il risultato

`docs/experiments/ai-01/results-human-<data>.md` con aggregati e senza input grezzi; commento di stato sull'issue.

## Se ti blocchi

Se `r0` non è disponibile, chiedi all'owner. Non usare un runtime cloud con chiave e non usare dati reali: la card è solo sintetica.
