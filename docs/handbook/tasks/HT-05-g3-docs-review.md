# HT-05 — Review della checklist G3 con occhi nuovi

| Campo | Valore |
|---|---|
| ID | HT-05 |
| Area | docs e pilota G3 |
| Tempo massimo | 2 h |
| Stato | pronta |
| Issue | da aprire con template `Human task` |

## Perché serve una persona

Chi entra in un progetto tecnico vede ambiguità che chi lo ha scritto non vede più. Serve qualcuno che legga il materiale del pilota come se fosse un partecipante.

## Obiettivo

Rendere comprensibile la [checklist G3](../g3/checklist.md) a chi non conosce il progetto, segnalando le frasi che non si capiscono o che promettono più di quanto è vero.

## Input e prerequisiti

- Checklist G3, roadmap, threat model e paper esterno
- Nessun dato reale: la decisione su G3 è ancora aperta

## Procedura

1. Leggi la checklist come se fossi un partecipante, non un manutentore.
2. Segna ogni frase che richiede una spiegazione aggiuntiva, con la domanda che ti sei fatto.
3. Controlla che non ci siano affermazioni di protezione o revoca presentate come guarantee prima del gate.
4. Controlla che il riquadro `DA DECIDERE` sia ancora presente e che il testo non lasci intendere che i dati reali siano già ammessi.
5. Proponi modifiche solo al testo: apri una PR se le correzioni sono chiare, altrimenti lascia un commento con `needs-owner`.

## Comandi

```sh
git checkout -b docs/g3-checklist-review
git commit -s -m "docs(handbook): clarify G3 checklist"
```

## Evidenze da conservare

- Elenco delle frasi ambigue con la domanda corrispondente
- Eventuale PR con le correzioni

## Criteri di accettazione

- [ ] Ogni ambiguità ha una domanda concreta, non un giudizio generico
- [ ] Il riquadro `DA DECIDERE` non è stato rimosso
- [ ] Nessun dato reale aggiunto per chiarire un esempio
- [ ] `ci-local` verde se è stata aperta una PR

## Dove finisce il risultato

Commento sull'issue per le domande; PR per le correzioni al testo.

## Se ti blocchi

Commenta con `needs-owner`: la decisione sui dati reali spetta all'owner, non a chi fa la review.
