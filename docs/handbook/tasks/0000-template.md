# HT-000 — Titolo della task

| Campo | Valore |
|---|---|
| ID | HT-000 |
| Area | scegli tra: prove dispositivi / test manuali / human eval / docs e G3 |
| Tempo massimo | 2 h |
| Stato | pronta / in corso / chiusa / bloccata |
| Issue | link all'issue di tracking |

## Perché serve una persona

Una frase: cosa non può fare l'agente al posto tuo (dispositivo fisico, giudizio soggettivo, dati reali, consenso).

## Obiettivo

Cosa deve essere vero alla fine della task. Un solo risultato verificabile.

## Input e prerequisiti

- Card e issue collegate
- HT-01 completata (ambiente verificato)
- Elenco dei file o comandi di partenza

## Procedura

1. Primo passo.
2. Secondo passo.
3. Ultimo passo: conserva il RAW e annota le anomalie.

## Comandi

```sh
comando --con --argomenti
```

## Evidenze da conservare

- Output grezzo dei comandi
- Versioni di OS, toolchain e dispositivo
- Eventuali screenshot o log di errore (solo dati sintetici)

## Criteri di accettazione

- [ ] Obiettivo raggiunto e verificabile
- [ ] RAW conservato e citato nel risultato
- [ ] Nessun dato personale nei file prodotti
- [ ] Blocchi segnalati con label `needs-owner`

## Dove finisce il risultato

Indica il posto esatto, secondo il [protocollo di handoff](../handoff.md).

## Se ti blocchi

Commenta con `needs-owner`, allega errore e contesto. Non proseguire inventando un risultato.
