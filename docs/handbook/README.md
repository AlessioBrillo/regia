# Handbook del collaboratore

Questa cartella è la parte **versionata** del materiale per chi entra nel progetto. Il paper di benvenuto, in linguaggio semplice, ha una [sorgente versionata](paper.md) da incollare in un Google Doc o esportare in PDF: `LINK AL DOC DA INSERIRE`. Se i due testi dissentono, **vale questo repo**: il Doc spiega, qui sono le regole.

## Stato

Regìa è al gate G1. Non esiste ancora un prodotto utilizzabile: le prove si fanno sugli eseguibili degli spike (`spikes/`) e sugli esempi di `core/domain`. Non cercare un'app da provare: non c'è.

## Ruolo

Il ruolo è **maintainer part-time** (2-4 ore a settimana). Le decisioni architetturali restano agli ADR accettati con evidenze: una prova produce evidenza, non ratifica una scelta.

## Indice

- [Paper di benvenuto](paper.md): testo semplice da incollare nel Google Doc o esportare in PDF
- [Protocollo di handoff](handoff.md): come passa il lavoro tra agente e persona, e dove finiscono i risultati
- [Rubrica per la valutazione AI](human-eval-rubric.md): come giudicare le risposte del modello
- [Task pronte](tasks/0000-template.md): modello e schede assegnabili; [HT-06](tasks/HT-06-slice-demo.md) è la demo end-to-end su dati sintetici
- [Checklist pilota G3](g3/checklist.md): materiale per partecipanti; la parte sui dati reali è ancora da decidere

## Regole che non si discutono

1. **Solo dati sintetici** finché il punto G3 non è deciso. Nessun dato personale in issue, PR, fixture, log o prompt.
2. Ogni numero riportato deve avere un comando, le versioni degli strumenti e il RAW grezzo.
3. Un errore di policy è `deny`: non trasformare un errore in un permissivo.
4. Le scelte aperte (sync engine, MLS, Cedar vs OPA, Temporal, client framework, provider AI) non si chiudono a voce: servono un esperimento e un ADR.
5. Ogni commit è firmato (GPG o SSH) e porta `Signed-off-by` (`git commit -s`).

## Primo giorno

1. Leggi [CONTRIBUTING.md](../CONTRIBUTING.md) e il paper esterno.
2. Prendi la card `HT-01` in `docs/handbook/tasks/`.
3. Segui la card, non improvvisare: se manca qualcosa, apri un'issue e chiedi.

## Se il paper e il repo non concordano

Aggiorna il repo e segnala la differenza: il paper esterno si aggiorna dopo, non prima.
