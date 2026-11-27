# HT-03 — Test manuale esplorativo sugli harness DAT-01

| Campo | Valore |
|---|---|
| ID | HT-03 |
| Area | test manuali |
| Tempo massimo | 2 h |
| Stato | pronta |
| Issue | da aprire con template `Human task` |

## Perché serve una persona

Gli harness producevano numeri, ma serve uno sguardo umano che noti un output assurdo, un errore strano o un canary fuori posto prima che il risultato diventi un ADR.

## Obiettivo

Eseguire gli scenari S-01, S-04, S-07 e S-08 disponibili e riportare anomalie e canary, senza cambiare le soglie.

## Input e prerequisiti

- HT-01 completata
- [DAT-01](../../experiments/DAT-01-data-model-sync.md) e le regole congelate negli incrementi
- Dataset sintetico generato in HT-02

## Procedura

1. Esegui il benchmark con 10 ripetizioni e conserva l'output grezzo.
2. Esegui il backlog S-08 con 10 ripetizioni e 1000 operazioni per client.
3. Per ogni riga, controlla che l'esito dichiarato corrisponda a quanto stampato: non fidarsi del riepilogo.
4. Cerca la stringa `CANARY-` negli output e nei file generati. Un canary visibile a un principal non autorizzato è una failure di sicurezza, non una nota.
5. Annota ogni caso in cui il comportamento non corrisponde alla regola congelata nella card DAT-01.
6. Non modificare soglie o regole per far passare un caso: un miss resta un miss.

## Comandi

```sh
cargo run -p regia-spike-dat01 --release --example bench -- 10
cargo run -p regia-spike-dat01 --release --example backlog -- 10 1000
```

## Evidenze da conservare

- Output grezzo di entrambi i comandi
- Elenco delle righe con esito anomalo
- Eventuali occorrenze di `CANARY-` con contesto

## Criteri di accettazione

- [ ] 10 ripetizioni eseguite per entrambi i comandi
- [ ] Ogni anomalia riportata con riga di output e comando che l'ha prodotta
- [ ] Canary trovati segnalati come security, con `needs-owner` immediato
- [ ] Nessuna soglia o regola modificata

## Dove finisce il risultato

`docs/experiments/dat-01/results-manual-<data>.md` e commento sull'issue. Per un canary, solo segnalazione privata secondo [SECURITY.md](../../../SECURITY.md): senza dettagli in un'issue pubblica.

## Se ti blocchi

Commenta con `needs-owner` e riporta il comando e la riga esatta.
