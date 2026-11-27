# Protocollo di handoff

Scopo: rendere esplicito chi fa cosa, così una prova non diventa un'opinione e un risultato non si perde.

## Il flusso

1. **Agente (prepara):** apre la task card con obiettivo, comandi esatti, dataset sintetico, template di risultato e criteri di accettazione.
2. **Persona (esegue):** segue la card sul proprio dispositivo, conserva il RAW, annota anomalie anche quando sembrano irrilevanti.
3. **Persona (registra):** scrive il risultato nel posto giusto (tabella sotto) e lascia un commento sull'issue con stato e blocchi.
4. **Agente (verifica):** controlla che numeri, comandi e versioni siano presenti; se mancano, chiede, non deduce.
5. **Owner (decide):** solo l'owner accetta un ADR o chiude una decisione aperta. La persona non ratifica scelte.

## Dove finisce il risultato

| Tipo di risultato | Dove | Esempio |
|---|---|---|
| Evidenza di una prova | `docs/experiments/<ID>/results-<data>.md` | `results-2026-11-26.md` |
| Stato, blocco, domanda | commento sull'issue, con label `needs-owner` | "bloccato: nessun client eseguibile" |
| Modifica a codice, docs o template | PR con titolo Conventional Commit | `docs(handbook): ...` |
| Dati del pilota G3 | report separato, fuori dal repo, finché la decisione non è presa | non allegare a una PR |

Le label `human-task`, `manual-test`, `needs-evidence` e `needs-owner` sono impostazioni GitHub, non file del repo: vanno create una volta sola.

## Cosa non fare

- Non scrivere "funziona" senza un numero o un comando che lo produca.
- Non cambiare una soglia per far passare una prova: un miss si riporta come miss.
- Non copiare input reali nei log, nelle fixture o nei prompt.
- Non aprire un'issue pubblica per un problema di sicurezza: usa [SECURITY.md](../../SECURITY.md).
- Non prendere una seconda card prima di aver chiuso la prima: il tempo disponibile è di 2-4 ore a settimana.
- Non chiudere una decisione aperta con un esito di prova: serve un ADR accettato.

## Check-in settimanale

Un check-in breve (15 minuti) serve a scegliere 1-2 card, chiarire le ambiguità e dichiarare i blocchi. Il resto passa dalle issue: la card è il contratto, il commento è lo stato.

## Definition of done di una task

- [ ] Card collegata all'issue o alla PR
- [ ] Comandi e versioni (OS, toolchain, dispositivo) registrati
- [ ] Ripetizioni minime della card eseguite e RAW conservato
- [ ] Casi di errore provati, oppure dichiarati non eseguibili con il motivo
- [ ] Nessun dato personale nei file prodotti
- [ ] Risultato nel posto giusto e commento di stato sull'issue
- [ ] Se la PR tocca il repo: `ci-local` verde, commit firmati e con `Signed-off-by`
