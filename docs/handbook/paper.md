# Benvenuto in Regìa

Questo è il paper per la persona che entra nel progetto per fare il lavoro pratico. È scritto in linguaggio semplice. Le regole lunghe e i dettagli tecnici stanno nella [cartella handbook](README.md): se qui e lì c'è una differenza, vale il repo.

Regìa è un progetto ai primi passi: siamo al gate G1. Non c'è ancora un'app da provare. Si lavora su esperimenti piccoli, chiamati **spike**, e ogni scelta importante deve essere dimostrata con una prova ripetibile.

## Cosa fa l'agente e cosa fai tu

L'agente (un'IA) sa scrivere codice, documenti e istruzioni. Non può fare alcune cose:

- non può premere i tasti su un dispositivo fisico;
- non può dare un giudizio umano su una risposta scritta male ma tecnicamente corretta;
- non può gestire il consenso di una persona reale;
- non può decidere una scelta architetturale: può solo preparare le evidenze.

Tu fai queste cose. Non devi sostituire l'agente: devi coprire quello che a lui manca. Il tuo ruolo è **maintainer part-time**, con 2-4 ore a settimana.

Una regola importante: una prova non ratifica una decisione. Le scelte aperte (per esempio il motore di sync, il client, i provider AI) diventano decisioni solo con un ADR accettato.

## Giorno 1: preparare l'ambiente

Obiettivo: avere due macchine pronte, Windows 11 e Arch Linux, e un gate verde.

1. Clona il repository su entrambe le macchine.
2. Configura Git. I comandi esatti sono nella card [HT-01](tasks/HT-01-setup.md). I punti da non dimenticare sono:
   - `git config core.hooksPath .githooks` (blocca commit non firmati);
   - `git config commit.gpgsign true` (firma i commit);
   - usa l'indirizzo `noreply` di GitHub;
   - firma ogni commit con `git commit -s`.
3. Installa il toolchain Rust: il file `rust-toolchain.toml` seleziona la versione giusta. Verifica con `cargo --version`.
4. Installa `just` e Docker.
5. Su Arch Linux esegui `just gate` (che chiama `ci-local`). Su Windows 11 esegui `ci-local.cmd` da PowerShell.
6. Scrivi in un commento sull'issue: versione del sistema operativo, versioni degli strumenti, esito del gate.

Se qualcosa non funziona, non aggirarlo: scrivi l'errore esatto in un commento con la label `needs-owner`.

## Come eseguire una prova

Ogni prova parte da una card in `docs/handbook/tasks/`. La card è un contratto: dice cosa fare, in quanto tempo e cosa devi conservare.

1. Apri l'issue della card e scrivi che la stai iniziando.
2. Leggi la card fino in fondo, compresi i criteri di accettazione.
3. Esegui i comandi così come sono scritti. Non cambiarli per ottenere un risultato migliore.
4. Ripeti il numero di volte indicato. Una sola prova non è una prova: serve un numero di ripetizioni.
5. Conserva l'output grezzo, chiamato **RAW**. Anche se la prova fallisce, il RAW è la parte più importante.
6. Prova anche i casi di errore: se non sono eseguibili, scrivi perché.
7. Se trovi un comportamento strano, segnalalo anche se sembra piccolo.

Ci sono due cose che non devi mai fare:

- non cambiare una soglia per far passare la prova: un insuccesso si riporta come insuccesso;
- non sostituire un dispositivo mancante con una simulazione e non dire "funziona" senza un comando che lo dimostri.

Oggi, per esempio, Android, iPhone e Mac non sono ancora disponibili. Le prove su quei dispositivi vanno segnate come "non eseguite", non come "superate".

## Come scrivere i risultati

Il risultato non è un riassunto a memoria. È un insieme di evidenze.

| Cosa | Dove finisce |
|---|---|
| Numeri e output grezzo di una prova | `docs/experiments/<ID>/results-<data>.md` |
| Stato, blocco o domanda | commento sull'issue, con label `needs-owner` |
| Modifica a codice o documenti | PR con titolo Conventional Commit |
| Materiale del pilota G3 | report separato, fuori dal repo, finché la decisione non è presa |

In ogni risultato devono comparire:

- il comando esatto che hai eseguito;
- la data e le versioni di OS e toolchain;
- il numero di ripetizioni;
- il RAW, o un link al RAW conservato;
- i casi che non hanno funzionato;
- i limiti della prova (per esempio: "tempo in processo, senza rete").

Il modello per scrivere un risultato è nel [protocollo di handoff](handoff.md). Per la valutazione delle risposte AI usa la [rubrica](human-eval-rubric.md): compila i primi 5 casi prima di guardare il punteggio dell'harness.

## Regole sui dati e sulla sicurezza

Queste regole non si discutono:

1. **Solo dati sintetici** finché la decisione sul pilota G3 non è chiusa. I dati sintetici sono generati dal repo e non riguardano persone reali.
2. Nessun dato personale in issue, PR, fixture, log o prompt. Se hai un dubbio, non allegare il dato: chiedi.
3. Alcuni oggetti sintetici hanno un titolo con la parola `CANARY-`. Se vedi un canary in un log, in una cache o in un output che non dovrebbe vederlo, è una failure di sicurezza, non un dettaglio.
4. Un errore di policy è sempre `deny`. Un errore non diventa mai un permesso.
5. Non scrivere codice crittografico fatto in casa: si usano librerie mantenute e revisionate.
6. Non mettere segreti o chiavi API nel repo. Vanno prese dall'ambiente.
7. Il materiale del pilota è in [checklist G3](g3/checklist.md), ma la parte sui dati reali è ancora **DA DECIDERE**: non comportarti come se fosse già autorizzata.

## Come segnalare un problema

- **Problema di sicurezza**: non aprire un'issue pubblica. Segui [SECURITY.md](../../SECURITY.md) e usa la segnalazione privata su GitHub.
- **Blocco su una task**: commento sull'issue con label `needs-owner`, errore esatto e contesto.
- **Documento poco chiaro**: scrivi la domanda che ti sei fatto, non solo "non capisco".
- **Dubbio su una decisione aperta**: chiedi all'owner. Non chiudere tu la decisione.

Il check-in è breve, circa 15 minuti a settimana: si scelgono una o due card, si chiariscono i dubbi e si dichiarano i blocchi. Il resto del lavoro è asincrono, sulle issue.

## Prima di chiudere

Controlla che valga tutto questo:

- [ ] ho eseguito i passi della card, senza inventare risultati
- [ ] ho conservato il RAW e registrato versioni e comandi
- [ ] ho provato i casi di errore o ho spiegato perché non era possibile
- [ ] non ho messo dati personali da nessuna parte
- [ ] il risultato è nel posto giusto e l'issue ha un commento di stato
- [ ] se ho aperto una PR, `ci-local` è verde e i commit sono firmati e con `Signed-off-by`
