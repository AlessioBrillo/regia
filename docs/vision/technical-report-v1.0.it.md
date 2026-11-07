# Regìa — Relazione tecnica di architettura, stack e workflow

**Versione:** 1.0  
**Data:** 20 ottobre 2026  
**Stato:** proposta tecnica per avvio delle prove; decisioni architetturali non ancora ratificate  
**Documento di riferimento:** *Regìa — Product Vision & Technical Experiment Charter v2.0*  
**Destinatari:** responsabile di progetto, principal software engineer, sviluppatori, specialisti AI, sicurezza e partecipanti alle prove  
**Ambito:** valutazione dello stack tecnologico, esperimenti di fattibilità, workflow applicativi e operativi, criteri di scelta, architettura MVP e traiettoria verso la visione completa  
**Esclusioni:** business plan, monetizzazione, go-to-market, pricing commerciale e obiettivi di vendita

> **Tesi della relazione.** Regìa va costruito come un sistema ambizioso, modulare e verificabile. Non conviene però fissare fin da subito ogni tecnologia candidata del documento di visione: conviene bloccare i principi architetturali e sottoporre le scelte più rischiose a prove comparative riproducibili. L'MVP iniziale deve essere una prima implementazione reale delle fondamenta della visione, non un prototipo usa-e-getta né una riduzione della visione stessa.

## Indice

- [0. Sintesi esecutiva](#0-sintesi-esecutiva)
- [1. Mandato, fonte e metodo](#1-mandato-fonte-e-metodo)
- [2. Requisiti architetturali](#2-requisiti-architetturali-derivati-dalla-visione)
- [3. Architettura logica target](#3-architettura-logica-target)
- [4. Valutazione dello stack tecnologico](#4-valutazione-dello-stack-tecnologico)
- [5. Workflow applicativi](#5-workflow-applicativi-da-implementare)
- [6. Perimetro tecnico dell'MVP](#6-perimetro-tecnico-dellmvp-iniziale)
- [7. Programma di esperimenti](#7-programma-di-esperimenti-tecnici)
- [8. Piano di test e criteri di accettazione](#8-piano-di-test-e-criteri-di-accettazione)
- [9. Qualità del software e delivery](#9-qualità-del-software-e-processo-di-delivery)
- [10. Pianificazione e governance tecnica](#10-pianificazione-ownership-e-governance-tecnica)
- [11. Rischi principali](#11-rischi-principali-indicatori-e-mitigazioni)
- [12. Mappatura M1–M17](#12-mappatura-dei-moduli-m1m17-alle-capacità-di-piattaforma)
- [13. Roadmap tecnica e gate](#13-roadmap-tecnica-con-gate-decisionali)
- [14. Decisioni preliminari e domande aperte](#14-decisioni-preliminari-e-open-questions)
- [15. Conclusione tecnica](#15-conclusione-tecnica)
- [16. Fonti tecniche e riferimenti](#16-fonti-tecniche-e-riferimenti)
- [Appendice A — Template ADR](#appendice-a--template-per-architecture-decision-record)
- [Appendice B — Checklist release](#appendice-b--checklist-release-sperimentale)

---

## 0. Sintesi esecutiva

Regìa combina proprietà che, prese singolarmente, sono affrontabili, ma che diventano difficili da integrare nello stesso sistema: dati personali e condivisi tra cerchie; funzionamento offline; sincronizzazione multi-dispositivo; confidenzialità e revoca dell'accesso; estrazione da input multimodali; modelli linguistici locali e remoti; workflow agentici durevoli; connettori esterni; audit comprensibile; utilizzo da parte di persone con competenze digitali differenti.

La raccomandazione è **non selezionare lo stack sulla base della popolarità di una libreria o della qualità di una demo AI**. La decisione deve derivare dai risultati di esperimenti che esercitino insieme il sistema dati, le policy, la sincronizzazione, il modello AI e il workflow. I componenti devono essere misurati in scenari realistici, e non solo valutati con benchmark generici.

### 0.1 Stack di riferimento provvisorio

| Dominio | Candidato di riferimento | Stato della decisione | Motivo principale / condizione da provare |
|---|---|---|---|
| App iOS | Swift + SwiftUI | **Raccomandazione provvisoria** | Accesso nativo a funzionalità OS e framework AI Apple ([APPLE-FM]); verificare la quantità di logica duplicata |
| App Android | Kotlin + Jetpack Compose | **Raccomandazione provvisoria** | Integrazione nativa con servizi Android e API GenAI on-device ([ANDROID-GENAI]); testare la matrice reale dei dispositivi |
| Core condiviso | Rust per moduli selezionati e ad alto valore di coerenza | **Da delimitare** | Policy domain, tipi, validazione, crittografia tramite librerie verificate e logica condivisa; non imporre Rust a ogni integrazione |
| Dati locali | SQLite con migrazioni versionate, FTS5 e indici separati | **Candidato forte** | Semplicità operativa, query e persistenza; la cifratura a riposo e il modello di proiezione vanno verificati |
| Offline-first / sync | Prova comparativa Automerge, modello relazionale sincronizzato e approccio ibrido | **Non ratificato** | È il principale rischio tecnico: convergenza, query, revoca, cancellazione, E2EE e carico storage devono essere misurati insieme |
| Crittografia tra membri | MLS (RFC 9420) tramite implementazione mantenuta e verificata, se l'integrazione multipiattaforma è praticabile | **Gate di fattibilità** | Il protocollo non risolve da solo identità, recovery, backup, autorizzazioni applicative e cifratura degli oggetti persistiti |
| Autorizzazioni | Policy dichiarative; prototipo comparativo Cedar vs OPA/Rego | **Da scegliere dopo spike** | Policy deny-by-default, testabilità, interoperabilità Rust/backend e leggibilità degli esiti |
| Backend | Modular monolith, API versionate, PostgreSQL e object storage UE | **Raccomandazione provvisoria** | Evita distribuzione prematura in microservizi e mantiene chiari i confini; i dati che il server non deve vedere non vanno inseriti nel suo modello in chiaro |
| Workflow durevoli | State machine esplicita + outbox nel primo verticale; spike Temporal prima di adottare la piattaforma | **Da decidere con test di recovery** | La durata dei workflow e le attese per approvazione sono reali; va però confrontato il valore di durable execution con il costo operativo |
| Agente | Plan → policy → approvazione → esecuzione tramite tool tipizzati | **Principio vincolante** | Il modello non può essere autorità di accesso né chiamare direttamente strumenti con effetti collaterali |
| AI on-device | Adapter per Apple Foundation Models, ML Kit / Gemini Nano e runtime open-weight dove compatibile | **Architettura multi-runtime** | Nessun runtime va assunto disponibile su tutti i device; ogni task deve poter degradare in modo dichiarato |
| AI cloud | Gateway multi-provider, routing basato su classificazione dati e capacità richieste | **Principio di portabilità** | Confrontare qualità, trattamento dati, latenza, stabilità, costo per workflow completato e disponibilità UE |
| Search | Ricerca SQL/FTS locale come base; vettori e hybrid search solo dopo benchmark | **Progressivo** | La ricerca semantica non deve sostituire filtri di accesso, query deterministiche o citazioni della fonte |
| Telemetria | OpenTelemetry con redaction e nessun contenuto personale nei segnali standard | **Raccomandazione** | Tracciamento end-to-end di workflow, policy, modelli, connettori ed esiti senza creare una seconda copia dei dati personali |
| Contratti esterni agentici | MCP come adapter di interoperabilità; A2A solo quando serve davvero comunicare con agenti esterni | **Non usare come modello di dominio** | I protocolli collegano componenti, ma non sostituiscono policy interne, identità e audit |

Questa tabella è una **baseline per l'esperimento**, non una dichiarazione che ogni componente sia già il migliore in assoluto. Le decisioni definitive saranno registrate in ADR (*Architecture Decision Records*) con i risultati e i limiti delle prove.

### 0.2 Le cinque decisioni da verificare per prime

1. **Modello dati e sync:** come rappresentare lo stesso fatto localmente, su più dispositivi e nelle diverse cerchie senza perdita, fuga di dati o incoerenze permanenti.
2. **E2EE e ciclo di vita dei membri:** come aggiungere, revocare e rimuovere una persona, ruotare le chiavi, gestire i dispositivi e ripristinare l'accesso senza promettere revoca retroattiva di dati già letti o copiati.
3. **Enforcement delle policy agentiche:** come garantire che nessun output del modello diventi un'azione esterna senza controllo deterministico, scope minimo, idempotenza e audit.
4. **Routing locale/cloud:** come selezionare il modello appropriato per ciascun task senza inviare contenuto sensibile a un sistema non autorizzato e senza assumere che i modelli on-device siano presenti o equivalenti.
5. **Workflow resiliente:** come recuperare dopo crash, rete assente, token scaduti, timeout, approvazioni tardive, errori parziali dei connettori e risultati esterni ambigui.

### 0.3 Esito atteso della fase tecnica

Al termine della valutazione, il progetto deve possedere: una matrice dei requisiti tecnici; benchmark e dataset di prova versionati; una build end-to-end utilizzabile su device reali; un insieme di ADR motivati; una threat model aggiornata; una suite di regressione; un inventario dei limiti noti; una roadmap tecnica per i moduli M1–M17 e A1–A15. Non è sufficiente una demo che riesce una volta.

---

## 1. Mandato, fonte e metodo

### 1.1 Fonte primaria

La base funzionale è il documento *Regìa — Product Vision & Technical Experiment Charter v2.0*, che mantiene i moduli M1–M17, i requisiti trasversali A1–A15, il modello dati, le cerchie, le etichette di confidenzialità, l'agente operativo, la modalità local-first, la multimodalità, i workflow, le integrazioni e le superfici. Questa relazione non sostituisce la visione funzionale; traduce i suoi obiettivi in ipotesi ingegneristiche, prove e decisioni.

### 1.2 Distinzione tra requisiti, ipotesi e prove

Ogni affermazione tecnica nel progetto deve appartenere a una delle categorie seguenti:

- **Requisito di visione:** comportamento necessario nel sistema completo.
- **Vincolo di progetto:** principio che non deve essere violato, ad esempio l'autorizzazione deterministica prima delle azioni.
- **Ipotesi tecnica:** implementazione candidata, da confrontare con alternative.
- **Criterio provvisorio:** soglia utile per decidere se proseguire; non è un risultato già conseguito.
- **Capacità dimostrata:** passata in test riproducibili, su una matrice definita e con evidenza salvata.
- **Limitazione nota:** caso non supportato, condizione degradante o rischio residuo accettato.

La distinzione è essenziale perché nel documento di visione sono presenti alcune soglie numeriche ambiziose (latenza vocale, accuratezze, disponibilità, consumo batteria, tempi di export). Devono essere trattate come obiettivi da convalidare mediante misure rappresentative, non come garanzie già disponibili nello stack.

### 1.3 Metodo decisionale

Per ciascuna scelta significativa, il team dovrà:

1. descrivere il bisogno e i vincoli di Regìa;
2. definire due o più alternative credibili;
3. fissare una prova che possa smentire la preferenza iniziale;
4. implementare un vertical slice minimo per le alternative selezionate;
5. eseguire gli stessi dati di test e gli stessi scenari d'errore;
6. registrare misure grezze, limiti, risultati e costi operativi;
7. decidere tramite ADR, includendo le condizioni che farebbero riaprire la decisione.

### 1.4 Principio di non-commercializzazione

La scelta tecnica non sarà guidata da prezzo di vendita, acquisizione clienti, conversione, churn o margine commerciale. Si misureranno invece: costo infrastrutturale per workflow, consumo dei modelli per task riuscito, risorse per sincronizzazione, complessità operativa, tempo di recovery, dipendenza dal provider, impatto su batteria e requisiti di manutenzione. Sono metriche ingegneristiche e di sostenibilità dell'esperimento, non un business plan.

---

## 2. Requisiti architetturali derivati dalla visione

Questa sezione riformula le esigenze di Regìa come proprietà verificabili del sistema.

| ID | Proprietà | Motivazione nella visione | Evidenza richiesta |
|---|---|---|---|
| AR-01 | Domini e contratti espliciti | M1–M17 condividono persone, eventi, task, fonti, permessi e azioni | Schema versionato, contratti API, migrazioni testate |
| AR-02 | Local-first reale | M6, M11 e requisiti di resilienza richiedono lettura/scrittura offline | Operazioni disponibili in modalità aereo; sincronizzazione successiva verificata |
| AR-03 | Convergenza deterministica | Modifiche concorrenti da più membri/dispositivi | Test property-based e simulazione di partizioni di rete; zero perdita dei casi definiti |
| AR-04 | Confini di cerchia applicati ovunque | M7–M8 richiedono permessi asimmetrici e oggetti riservati | Test negativi su API, sync, search, cache, notifiche, esportazioni, prompt e audit |
| AR-05 | Cifratura coerente con il modello minacce | Dati personali, segreti e condivisioni tra cerchie | Threat model, key lifecycle, test di revoca, rotazione e recovery |
| AR-06 | Provenienza della conoscenza | Risposte e azioni devono essere spiegabili | Ogni fatto estratto mantiene fonte, posizione, timestamp, confidence e stato di conferma |
| AR-07 | Agente governato | M9, M8 e requisiti trasversali richiedono autonomia proporzionata | Ogni tool call mutante ha policy decision, autorizzazione, stato e audit correlabili |
| AR-08 | Workflow riprendibili | Promemoria, import documenti, approvazioni e integrazioni possono durare a lungo | Iniezione di crash/timeout; ripresa senza duplicare effetti collaterali |
| AR-09 | Provider AI sostituibili | Stack ibrido locale/cloud e dipendenze esterne variabili | Interfaccia comune, test contrattuali e benchmark per task |
| AR-10 | Degrado esplicito | Funzioni AI e connettori non sono sempre disponibili | Stato UI riconoscibile; nessun falso successo; alternative non-AI sicure |
| AR-11 | Portabilità dei dati | Export, correzione e cancellazione sono richiesti | Export/import verificato, test di eliminazione e consistenza tra proiezioni |
| AR-12 | Osservabilità senza esfiltrazione | Diagnosi del sistema e audit agente | Trace correlabili senza payload personali per default; accesso controllato agli audit |
| AR-13 | Evoluzione modulare | La visione resta ampia e continuerà ad evolvere | Bounded context, API interne, ownership dei dati e migrazioni retrocompatibili |
| AR-14 | Esperienze diverse, regole comuni | Profili Semplice/Completo/Avanzato e persone diverse | Stesse policy e stessa fonte di verità; interfacce non possono aggirare le policy |
| AR-15 | Sicurezza verificabile | Minori, salute, finanza e dati legali alzano il rischio | Threat modeling, suite abuse, test di isolamento e revisione indipendente prima di espandere le prove |

### 2.1 Requisiti che necessitano di precisazione prima dell'implementazione

Alcune formulazioni della visione richiedono una specifica tecnica più precisa:

- **“Zero perdita”** deve dichiarare quali operazioni, tipi di dato e conflitti sono coperti; non può essere una promessa assoluta astratta. Va testata una matrice di scenari e verificata la conservazione di ogni intenzione utente.
- **“Revoca immediata”** può impedire accessi futuri tramite server e chiavi aggiornate, ma non può cancellare copie o informazioni già viste da un membro. La UX e il modello minacce devono dirlo chiaramente.
- **“E2EE e AI cloud”** sono compatibili soltanto se si definisce esplicitamente quali contenuti vengono decifrati, da chi, dove, per quale scopo e con quale consenso. Un server che deve generare contesto in chiaro non è cieco a quel contenuto.
- **“Classificazione della sensibilità recall ≥95%”** richiede tassonomia, dataset rappresentativo, gestione dei falsi negativi e una policy fail-safe. Una classificazione del modello non deve essere l'unico controllo per inviare dati al cloud.
- **“SLO 99,9% notifiche critiche”** richiede di definire evento misurato, dipendenze esterne, finestra, esclusioni e comportamento quando push/telefonia non sono controllabili da Regìa.
- **“Costo AI come quota del ricavo”** non è pertinente alla fase attuale senza obiettivi di vendita. Sostituirlo con budget sperimentale per workflow e misure di costo/risorse per esito riuscito.
- **“Ogni dato in UE”** è una promessa da verificare per ogni provider, sub-responsabile, log, supporto, backup, inferenza e telemetria. La regione nominale non basta a provare da sola la residenza di tutti i trattamenti.

---

## 3. Architettura logica target

### 3.1 Principio: sistema a livelli, non una collezione di agenti

Regìa va modellato prima di tutto come un sistema applicativo di dominio con workflow governati; l'AI è una capacità invocata dal sistema, non l'architettura che sostituisce il sistema. Il core applicativo conserva il controllo su identità, policy, stato, persistenza, sincronizzazione, audit e side effect.

### 3.2 Vista logica

```text
Superfici
  iOS / Android / Web futuro / widget / share sheet / voce / email / notifiche
        |
        v
Application Facade (comandi e query tipizzati)
        |
        +--> Identity & Circle Context
        +--> Policy Decision / Policy Enforcement
        +--> Domain Services (calendar, tasks, routines, documents, memory...)
        +--> Workflow Runtime (durable state, retries, timers, approvals)
        +--> AI Gateway (local/cloud router, model adapters, evaluations)
        +--> Connector Runtime (OAuth, sync mapping, rate limits, webhooks)
        +--> Audit / Provenance / Observability
        |
        +--> Local persistence + search projections + local encrypted blobs
        +--> Sync protocol + encrypted replication / server relay
        +--> Remote API + PostgreSQL + object storage + KMS where appropriate
```

### 3.3 Confini architetturali

| Confine | Responsabilità | Non deve fare |
|---|---|---|
| UI/superfici | Presentare, raccogliere input, mostrare fonte/stato e chiedere conferme | Decidere autonomamente autorizzazioni o modificare direttamente dati remoti senza application facade |
| Core di dominio | Tipi, invarianti, validazione, transizioni di stato, logica condivisa | Dipendere da uno specifico LLM o provider cloud |
| Policy enforcement | Autorizzare o negare operazioni in base a principal/action/resource/context | Delegare la decisione finale al modello linguistico |
| AI Gateway | Invocare il runtime appropriato e normalizzare output | Avere accesso illimitato a tutto il grafo o nascondere la destinazione dei dati |
| Workflow runtime | Gestire stato, retry, timer, attese umane, idempotenza e recovery | Trattare retry come prova che l'azione esterna non sia già avvenuta |
| Connector runtime | Adattare API esterne, token, rate limit e riconciliazione | Ricevere token o scope più ampi del necessario |
| Local store | Persistenza locale e query; eventualmente proiezioni indicizzate | Diventare una copia in chiaro non governata dei dati E2EE |
| Sync/relay | Trasportare e riconciliare aggiornamenti autorizzati | Essere implicitamente trusted per l'accesso al plaintext dei dati cifrati end-to-end |
| Audit | Registrare decisioni, fonti, attore, versione policy e risultato | Salvare automaticamente tutti i payload sensibili senza minimizzazione |

### 3.4 Architettura MVP e architettura target

L'MVP deve già rispettare questi confini. Può implementare una quantità limitata di connettori e workflow, ma non deve introdurre un percorso che in futuro renda impossibili E2EE, policy deterministiche, idempotenza o provenienza. L'architettura target potrà avere più runtime, nodi, superfici e connettori; la base non deve essere riscritta per ottenerli.

Il principio operativo è un **modular monolith sul backend** finché i carichi o i confini di sicurezza non giustifichino la separazione fisica. Il modular monolith non vuol dire monolite privo di confini: i moduli hanno interfacce, ownership e test indipendenti. Si separano servizi e processi quando emergono ragioni misurabili (scalabilità, isolamento, affidabilità, librerie incompatibili o requisito di deploy indipendente), non per anticipare una topologia teorica.

---

## 4. Valutazione dello stack tecnologico

### 4.1 Client: nativo vs cross-platform

#### Alternativa A — iOS SwiftUI + Android Jetpack Compose

**Punti di forza:** integrazione diretta con i sistemi operativi, notifiche, share sheet, permessi, secure storage e API locali AI; debug vicino al device; possibilità di adottare tempestivamente API specifiche.

**Rischi:** duplicazione di parte della logica UI e d'integrazione; diverse cadenze di rilascio; necessità di mantenere comportamenti coerenti.

#### Alternativa B — Flutter o React Native

**Punti di forza:** velocità per interfacce condivise, ecosistema, possibilità di riusare una porzione ampia del frontend.

**Rischi per Regìa:** bridge verso funzioni OS, diverse API AI native, voice in background e comportamenti offline possono aumentare il codice specifico per piattaforma. Il vantaggio va misurato su una prova con share sheet, storage protetto, notifica, dettatura e un task AI on-device reale.

#### Alternativa C — core Rust + superfici native

È la proposta più coerente quando servono tipi, invarianti e logica identici, ma **non bisogna presumere che tutto debba essere condiviso in Rust**. Framework AI, permessi, UI, notifica, lifecycle e integrazioni di sistema resteranno spesso nativi. Le interfacce con il core possono usare binding C/UniFFI o un protocollo interno versionato.

**Raccomandazione provvisoria:** mantenere nativi i client principali e usare Rust soltanto per i componenti la cui condivisione porta un beneficio misurabile: validazione e transizioni di dominio, policy adapter, logica di conflitto, tipi portabili e primitive applicative testabili. Prima di allargare il core, misurare costo dei binding, crash debugging, dimensione binaria, velocità di sviluppo e test su iOS/Android.

**Spike CL-01 — prova comparativa client.** Implementare un solo flusso: acquisisci testo o PDF da share sheet, persisti localmente, mostra la proposta strutturata, conferma o annulla, sincronizza e gestisci un errore di rete. Confrontare soluzione nativa con soluzione cross-platform per tempo e complessità di integrazione, stabilità, accessibilità, latenza percepita, storage e qualità del debugging. Non usare un'app demo che non includa i casi di errore.

### 4.2 Core condiviso e linguaggi

**Candidato:** Rust per logica condivisa ad alta criticità e per alcune componenti backend; Kotlin e Swift per l'integrazione di piattaforma; eventualmente Python per pipeline sperimentali AI/offline evaluation, isolato dietro contratti espliciti.

Il core Rust non deve assorbire indiscriminatamente OCR, inferenza, SDK vendor, connettori e UI. I confini consigliati sono:

- `regia-domain`: tipi, invarianti, versionamento oggetti e regole di transizione;
- `regia-policy-adapter`: costruzione del contesto di autorizzazione e interpretazione dei risultati, senza sostituire il motore scelto;
- `regia-sync-adapter`: adapter al motore di sync selezionato, una volta validato;
- `regia-provenance`: struttura per fonti, estrazioni, versioni, conferme e citazioni;
- `regia-action-contracts`: schemi delle azioni proposte, eseguite e riconciliate;
- `regia-crypto-adapter`: confine su librerie crittografiche mantenute, senza implementazione crittografica proprietaria.

Ogni crate deve avere API pubblica minima, test unitari, fuzz/property tests dove opportuno, dipendenze e licenze inventariate. Evitare crate interni che contengono soltanto una riga di wrapper senza reale isolamento o riuso.

### 4.3 Storage locale e modello dati

#### Candidati

1. **SQLite + schema relazionale versionato:** forte base per query, filtri, transazioni e proiezioni locali; FTS5 può essere valutato per la ricerca testuale ([SQLITE-FTS5], [SQLITE-PRAGMA]).
2. **Automerge come struttura CRDT per gli oggetti collaborativi:** vantaggi per merge e operatività local-first, ma richiede di gestire query, indice, migrazioni, dimensione e integrazione con il ciclo di vita delle chiavi.
3. **Storage relazionale con API server e motore di sync dedicato:** più familiare in alcuni stack, ma deve dimostrare offline-write, merge, isolamento e comportamento con dati cifrati end-to-end.
4. **Approccio ibrido:** CRDT come sorgente di verità per un perimetro di oggetti collaborativi e SQLite come proiezione/query locale, oppure SQL come fonte di verità con log/versioni espliciti. L'ibrido deve definire chiaramente l'autorità di ogni dato.

#### Raccomandazione

**Non ratificare ancora Automerge + SQLite come soluzione finale.** Automerge è una candidatura seria perché nasce per applicazioni collaborative local-first e dispone di un protocollo di sync documentale ([AUTOMERGE-CONCEPTS], [AUTOMERGE-REPO]). Questo però non prova che sia l'abbinamento migliore per il grafo di Regìa, che include oggetti relazionali, confidenzialità granulari, query, revoche, dati esterni e indicizzazione.

Lo spike deve rispondere a quattro domande fondamentali:

- Qual è l'unica fonte di verità per ogni oggetto: documento CRDT, tabella SQL o evento di dominio?
- Come si mantiene l'indice locale coerente con la fonte e come viene rigenerato dopo corruzione o migrazione?
- Come si impedisce che un indice, una notifica, una cache o un risultato di ricerca riveli metadati non autorizzati?
- Come si comportano il merge e la cancellazione quando un membro è stato rimosso o non possiede più le chiavi?

#### Benchmark DAT-01 — modello dati e sync

Preparare dataset riproducibili con persone, cerchie annidate, task, eventi, documenti e fonti. Simulare almeno due e cinque client, modifiche offline, modifiche concorrenti dello stesso oggetto, delete/restore, migrazioni di schema, accessi simultanei e ritorno in rete. Confrontare:

- operazioni persistite e intenzioni utente preservate;
- conflitti risolti automaticamente vs conflitti che richiedono intervento;
- tempo di convergenza e bytes trasferiti;
- tempo per query tipiche e ricostruzione indici;
- storage totale e crescita della storia;
- comportamento dopo crash in fase di scrittura;
- correttezza delle cancellazioni e isolamento tra cerchie;
- complessità di debugging e implementazione.

**Gate:** non scegliere in base a un benchmark sintetico di performance. È necessario superare tutti i test di sicurezza e consistenza; solo dopo si confrontano latenza e costi operativi.

### 4.4 Sincronizzazione local-first

La sync è un protocollo applicativo, non soltanto una libreria. Lo stack dovrà specificare:

- identità stabile di oggetti, cerchie, dispositivi e operazioni;
- ordine causale ove necessario e gestione degli aggiornamenti concorrenti;
- semantica di delete, undo, tombstone e retention;
- versionamento e migrazione di schema;
- replay dei messaggi e deduplicazione;
- throttling, batching, ripresa e backpressure;
- cifratura, autenticazione del peer e autenticazione del relay;
- policy applicate sia al fetch sia alla materializzazione locale;
- comportamento in caso di revoca durante un client offline.

Una CRDT può risolvere conflitti di convergenza tra repliche, ma **non** decide quale modifica è autorizzata, non implementa da sola le regole semantiche del dominio e non elimina i rischi di leakage attraverso metadati, indici o notifiche.

### 4.5 Identità, cerchie, autorizzazioni e crittografia

#### Modello di autorizzazione

M7–M8 richiedono più di un semplice ruolo. Il sistema deve distinguere almeno:

- principal: persona, dispositivo o identità di servizio;
- action: read, discover existence, create, update, delete, export, share, execute;
- resource: oggetto e relazioni correlate;
- context: cerchia, ruolo, etichetta, scopo, canale, livello di autonomia, stato del consenso e contesto temporale quando appropriato.

**Deny-by-default.** Se il motore di policy è irraggiungibile, se mancano attributi essenziali o se la decisione è in errore, le azioni ad alto rischio devono essere negate e quelle a basso rischio possono soltanto proseguire secondo fallback esplicitamente definito e non mutante. Mai interpretare un errore come `allow`.

#### Cedar vs OPA/Rego

- **Cedar**: candidato quando la priorità è un linguaggio dedicato all'autorizzazione, con separazione tra policy e logica applicativa e un modello di principal/action/resource/context leggibile ([CEDAR]).
- **OPA/Rego**: candidato quando serve un policy engine general-purpose e si vuole una valutazione dichiarativa su documenti strutturati, anche al di fuori del dominio applicativo ([OPA]).

Non eleggere il motore prima di una prova che includa policy annidate di Regìa, test generati, explainability della decisione, versionamento, prestazioni, binding dal backend e, se necessario, esecuzione locale. Il motore non deve accedere da solo a dati non autorizzati: è l'application layer che prepara un input minimo e completo.

#### Crittografia di gruppo

MLS è uno standard IETF (RFC 9420) concepito per l'istituzione asincrona di chiavi di gruppo con forward secrecy e post-compromise security ([MLS]). Questo lo rende un candidato pertinente, non una soluzione completa al problema di Regìa.

Lo spike CRY-01 deve provare: creazione cerchia, invito, accettazione, aggiunta di un nuovo dispositivo, rimozione membro, rotazione chiavi, messaggi in transito, dati persistiti, offline al momento della revoca, backup, recovery e restore. Per ogni caso si devono distinguere accesso futuro, accesso a dati storici, copie precedentemente scaricate e metadata residui.

**Regole:** usare librerie mantenute e revisionate; non sviluppare algoritmi crittografici proprietari; documentare le primitive e le versioni; verificare i limiti della libreria su iOS, Android e server; progettare key backup/recovery come funzione critica, non come dettaglio UX.

#### Cifratura applicativa e indicizzazione

Occorre un diagramma esplicito di dove esiste plaintext: dispositivo, RAM del processo, servizio di sync, orchestratore, log, provider AI, OCR/STT, object store e backup. “Server senza accesso al contenuto” deve essere vero per tutto il percorso dichiarato, non soltanto per una tabella di database.

Le ricerche devono filtrare per autorizzazioni **prima** di restituire risultati o snippet. Se serve generare un embedding remoto, l'invio di testo sorgente al servizio remoto è trattamento di contenuto e va autorizzato; l'embedding stesso può rivelare informazioni e resta da proteggere. Non considerare un vettore “anonimo” per definizione.

### 4.6 Backend e persistenza remota

#### Baseline proposta

- API HTTPS versionate, contratti tipizzati e autenticazione esplicita;
- modular monolith con moduli di dominio isolati;
- PostgreSQL per dati operativi che il server è autorizzato a conoscere, metadati di connettori, coordinamento e workflow;
- object storage compatibile S3 in regione UE per payload che il backend è autorizzato a conservare, cifrati e con policy di retention;
- KMS/HSM o servizi equivalenti per segreti e chiavi server-side;
- transactional outbox per pubblicare eventi in modo consistente con le transazioni;
- migrazioni versionate e procedure testate di backup/restore;
- nessun contenuto personale in log applicativi per impostazione predefinita.

PostgreSQL non deve diventare la “fonte canonica” di dati che in un'architettura E2EE il server non dovrebbe interpretare. Si deve dichiarare, oggetto per oggetto, se i dati sono server-readable, end-to-end encrypted o solo temporaneamente elaborati da un worker autorizzato.

#### Perché non microservizi subito

L'MVP ha bisogno di verificare confini di dominio, dati, sicurezza, sync e workflow prima di dover gestire discovery, deployment distribuiti, versioni multiple, tracing cross-service e failure mode inter-servizio. Separazioni future sono ammesse e previste, ma vanno giustificate da isolamento o carichi reali. Gli adapter AI e connettori possono già essere processi separati se richiedono dipendenze o trust boundary specifici.

### 4.7 Workflow engine: macchina a stati vs Temporal

Regìa ha workflow che possono durare millisecondi oppure ore/giorni: estrazione documento, richiesta di approvazione, reminder schedulato, invito di un membro, sincronizzazione dopo assenza rete, rinnovo token, azione esterna e riconciliazione del suo esito. Per queste attività la durata e il recovery sono requisiti reali.

#### Alternativa A — state machine applicativa + outbox + worker

**Pro:** stack ridotto, controllo diretto, più semplice per primi verticali e job brevi.  
**Contro:** retry, timer durevoli, attese umane, recupero da crash, cancellazioni e osservabilità vanno implementati e mantenuti dal progetto.

#### Alternativa B — Temporal o runtime di durable execution equivalente

**Pro:** progettato per riprendere workflow dopo crash, timeout e attese di approvazione ([TEMPORAL]).  
**Contro:** nuova componente operativa, modello di programmazione specifico, migrazioni/versioning dei workflow, bisogno di capire la persistenza delle history e la gestione dei payload sensibili.

#### Raccomandazione

Non introdurre Temporal soltanto perché esiste un agente. Fare lo spike WF-01 sul workflow più rischioso e realistico: *input → estrazione → approvazione → side effect esterno simulato → timeout → retry → riconciliazione*. Confrontare una state machine persistente con Temporal.

**Criteri per adottare Temporal:** recovery e timers molto più semplici da dimostrare; supporto adeguato per attese umane e versionamento; payload sensibili mantenuti fuori dalla history quando non necessari; operatività e deployment accettabili; idempotenza degli effetti esterni comunque garantita. Se le esigenze di durable execution sono già centrali e i test confermano il valore, Temporal è un forte candidato per il backend target.

### 4.8 Architettura AI: task router e adapter, non dipendenza da un modello

#### Contratto comune

Ogni invocazione AI dovrebbe dichiarare almeno:

```text
TaskRequest {
  task_type, input_ref, data_classification, allowed_data_scope,
  required_capabilities, latency_budget, offline_required,
  locale, output_schema_version, retention_policy
}

TaskResult {
  typed_output, source_refs, model/provider/version,
  confidence_or_quality_signals, validation_status,
  latency, resource_usage, warnings, fallback_reason
}
```

I campi sono concettuali e vanno concretizzati in uno schema versionato. Il `confidence` fornito da un LLM non è una probabilità calibrata salvo validazione; perciò non va usato da solo per autorizzare azioni.

#### Famiglie di task

| Famiglia | Strategia primaria | Fallback | Vincolo |
|---|---|---|---|
| Parsing intent e classificazione semplice | Regole/classificatore piccolo, on-device quando utile | UI di chiarimento | Basso costo di errore; nessuna scrittura automatica non verificata |
| Estrazione da documento/input condiviso | OCR + parser/LLM strutturato; locale se qualità sufficiente | Bozza correggibile manualmente | Ogni campo importante conserva evidenza e offset/pagina quando possibile |
| Q&A su dati di Regìa | Query deterministiche + retrieval autorizzato + risposta citata | “Non ho trovato dati sufficienti” | La retrieval pipeline deve filtrare accesso prima del contesto del modello |
| Pianificazione complessa | Cloud model o modello locale sufficiente, in base al budget dati | Scomposizione in decisioni più piccole / chiarimento | Il piano rimane una proposta non eseguibile fino a policy gate |
| Classificazione sensibilità | Segnali deterministici + modello locale dove possibile + regola prudenziale | Tratta come più sensibile / richiedi conferma | Un falso negativo non deve aprire il canale cloud automaticamente |
| Voce streaming | STT/TTS a cascata o speech-to-speech dopo confronto | UI testuale / funzione offline limitata | Trascrizione, destinazione dati e consenso espliciti |
| Routine e previsione | Statistica/regole prima dell'LLM | Suggerimento disabilitabile | Le previsioni non sono fatti; nessuna inferenza senza spiegazione |

#### Routing locale/cloud

Il router deve operare su metadati del task e policy prima di serializzare il prompt. Flusso consigliato:

1. classificazione preliminare dei dati con regole conservative;
2. calcolo del perimetro minimo di dati consentito;
3. verifica se il task richiede offline o un runtime specifico;
4. selezione tra motori autorizzati compatibili con il livello di sensibilità;
5. minimizzazione e redazione dei campi non necessari;
6. invocazione con output strutturato;
7. validazione schema e fonti;
8. eventuale fallback esplicito, mai silenzioso se cambia trattamento dati;
9. audit della scelta del router senza registrare il prompt personale per default.

Il fallback da locale a cloud non può essere automatico se introduce un diverso trattamento dei dati o supera la classificazione consentita. In tal caso il sistema deve fermarsi o chiedere consenso specifico.

#### Prove comparative AI

Il modello migliore non è quello con un singolo punteggio elevato. Per ciascun task si misura: correttezza strutturata, errori pericolosi, completezza, citazioni, latenza p50/p95, stabilità tra versioni, costo/risorse per risultato corretto, memoria, consumo energetico, funzionamento offline e condizioni d'uso/retention del provider. Il dataset deve includere casi facili, ambigui, avversariali e fuori distribuzione.

Modelli e framework citati nella visione — Apple Foundation Models, ML Kit/Gemini Nano, runtime open-weight (ad esempio llama.cpp o ONNX Runtime ove compatibili) e provider cloud europei o globali — vanno trattati come adapter candidati. Disponibilità, termini, capacità e risultati devono essere verificati su device e versioni concrete. Evitare un adapter universale sovra-astratto prima di aver implementato almeno due runtime reali.

### 4.9 Ricerca e second brain

M12 richiede un grafo tipizzato, fonti e relazioni. L'ordine consigliato è:

1. query deterministiche sul modello dati e filtri di accesso;
2. full-text search locale sui contenuti autorizzati;
3. indicizzazione semantica per i casi in cui la ricerca lessicale è insufficiente;
4. ricerca ibrida (keyword + vettoriale + filtri + reranking) dopo aver definito benchmark e misurato il recall;
5. risposta generata con citazioni alle fonti e distinzione tra dati espliciti, inferenze e contenuti mancanti.

I vettori non sono la fonte di verità. Devono poter essere rigenerati e cancellati insieme alle fonti. Filtri per cerchia e confidenzialità devono essere applicati prima che i risultati siano esposti al modello. Il test di retrieval deve includere query a cui l'utente non ha diritto di ottenere risposta: il risultato corretto è nessuna rivelazione, non una risposta plausibile.

### 4.10 OCR, documenti, voce e cattura multimodale

La pipeline media deve separare:

- acquisizione e verifica del formato;
- scansione antivirus/sandbox dove appropriato;
- OCR/STT e pre-processing;
- estrazione strutturata;
- collegamento ai passaggi della fonte;
- revisione umana dei campi ad alto impatto;
- creazione degli oggetti solo dopo validazione/approvazione secondo il workflow;
- retention del file originale e delle versioni derivate;
- cancellazione coerente di testo estratto, chunk, embedding, cache e file temporanei.

Per documenti amministrativi italiani, date, codici fiscali, importi e scadenze devono passare validazioni deterministiche e controlli di plausibilità. Non basta “l'LLM pensa che la scadenza sia il 30 novembre”. La data estratta va collegata al testo/pagina in cui appare e deve essere distinguibile da una data di emissione o di pagamento.

Per la voce è raccomandata la prova di una cascata **STT → intent/LLM → policy → esecuzione → TTS**: l'intermedio testuale consente audit, disambiguazione e controllo prima di produrre effetti. Va confrontata con un modello speech-to-speech per latenza, interruzioni, qualità in italiano, code-switching, rumore e costo/risorse. La latenza indicata nella visione è un obiettivo da misurare end-to-end e non la latenza dichiarata di un singolo stadio.

### 4.11 Connettori esterni e interoperabilità agentica

I connettori sono boundary di sicurezza e di affidabilità. Ogni connettore deve avere:

- scope minimo e credenziali conservate in secret store;
- rotazione/revoca token, gestione scadenze e consenso;
- mapping tra id remoto e oggetto interno;
- paginazione, rate limit, retry con backoff e checkpoint;
- webhook firmati e/o polling idempotente;
- riconciliazione tra lo stato locale e quello remoto;
- gestione di errori parziali e API non disponibili;
- log tecnico redatto e audit delle azioni;
- test contract contro dati simulati e, successivamente, ambiente reale di test.

**MCP** va usato come protocollo di interoperabilità per collegare strumenti e contesti ([MCP-SPEC], [MCP-AUTH]), quando riduce davvero il costo d'integrazione. Ogni server MCP rimane un boundary potenzialmente non attendibile: tool schema, descrizioni e contenuti provenienti da esso non autorizzano azioni. Le chiamate passano dal registry interno, dalla validazione degli argomenti e dal policy enforcement di Regìa. **A2A** è un'opzione futura per parlare con agenti esterni ([A2A]); non serve a coordinare i moduli interni finché una comunicazione tra agenti distinti non sia un requisito dimostrato.

Le integrazioni con banche, PA, telefonia e servizi che richiedono accreditamenti o autorizzazioni vanno pianificate su canali ufficiali, con legale/compliance quando opportuno e senza presumere che un endpoint tecnicamente raggiungibile sia autorizzato all'uso.

### 4.12 Security, privacy e governance AI

#### Threat model minimo

Il threat model deve includere almeno:

- account compromesso e furto/disponibilità del device;
- dispositivo perso o revocato ma rimasto offline;
- membro di una cerchia malevolo o che esce dal gruppo;
- accesso trasversale tra cerchie tramite cache, ricerca, inferenza o notifica;
- token OAuth rubato o con scope eccessivo;
- prompt injection in email, PDF, inviti e pagine web;
- tool poisoning o descrizioni ingannevoli di tool MCP;
- output AI non valido, inventato o ambiguo;
- duplicate execution a seguito di timeout;
- provider indisponibile, retention diversa da quella attesa o routing errato;
- backup e log che contengono dati cancellati o non cifrati;
- abusi da parte di utenti con privilegi legittimi ma eccessivi;
- perdita, corruzione o restore parziale dei dati.

#### Controlli per l'agente

- I contenuti importati sono dati non attendibili, non istruzioni di sistema.
- Il modello riceve solo contesto autorizzato e minimizzato per quel task.
- I tool hanno schema tipizzato, scope minimi, allowlist e timeout.
- Le azioni mutanti passano da policy enforcement fuori dal modello.
- L'azione proposta è separata dall'azione eseguita.
- L'approvazione è vincolata a un riepilogo preciso: cosa, verso chi, su quali dati, con quale effetto.
- La conferma non può essere riutilizzata per una versione diversa del piano o del destinatario.
- Ogni side effect riceve un idempotency key e un execution id.
- Per esiti ambigui si riconcilia lo stato remoto prima di ritentare.
- Il “guardiano” può segnalare o negare, ma non ha credenziali di scrittura né l'ultima parola sulla policy.
- Le decisioni di policy e i risultati AI restano distinguibili nell'audit.

#### Privacy e osservabilità

OpenTelemetry è un candidato appropriato per correlare trace, metriche e log ([OTEL]), ma la configurazione di Regìa deve evitare che il payload dei dati personali diventi telemetria. Gli attributi predefiniti non devono includere testo, nome, documento, prompt o token. Gli identificativi sono pseudonimi rotabili; l'accesso agli audit è autorizzato e auditable. La telemetria AI deve consentire di correlare versione del modello, task, latenza, esito di validazione e motivo del fallback senza duplicare automaticamente contenuti dell'utente.

Il programma di sicurezza deve prendere come riferimento OWASP ASVS per l'applicazione ([OWASP-ASVS]) e OWASP Top 10 for LLM Applications per i rischi agentici ([OWASP-LLM]), adattandoli al threat model. Il NIST AI RMF ([NIST-AI-RMF]) e il suo profilo per la GenAI ([NIST-GENAI]) sono riferimenti di governance del rischio, non certificazioni automatiche né sostituti della valutazione giuridica.

---

## 5. Workflow applicativi da implementare

I workflow sono il vero prodotto operativo di Regìa. Devono essere specificati come macchine a stati con input, output, autorizzazioni, timeout, compensazioni, fonti e criteri di completamento. Non devono essere implementati come prompt lunghi che “sperano” di arrivare a un risultato.

### 5.1 WF-01 — Cattura universale → proposta strutturata

**Attivazione:** share sheet, voce, testo, foto, PDF o email inoltrata.  
**Risultato:** proposta di task/evento/documento/scadenza, con fonte e campi da confermare.  
**Non-obiettivo:** creare automaticamente eventi o inviare comunicazioni soltanto perché un LLM ha estratto un'intenzione.

#### Stati

`received → quarantined/validated → classified → extracted → linked_to_sources → policy_checked → needs_review | ready_for_confirmation → accepted | corrected | rejected → committed → indexed → synced`

Ogni passaggio deve poter fallire in modo esplicito e riprendersi senza duplicare gli oggetti.

#### Controlli

- limitazione dimensione/formato e protezione da file malevoli;
- classificazione prudente della sensibilità;
- trattamento dei contenuti come input non attendibile;
- output strutturato validato da schema;
- fonte allegata a ciascun campo importante;
- distinzione tra certezza, ipotesi e dato mancante;
- proposta correggibile prima del commit;
- nessuna scrittura fuori dallo scope autorizzato;
- commit idempotente e audit correlato.

#### Acceptance candidate

- Il 100% degli oggetti creati dal workflow conserva la fonte o un motivo esplicito per cui la fonte manca.
- I campi obbligatori non validati non entrano come fatti confermati.
- Retry di un input identico non crea oggetti duplicati.
- Le prove di confusione tra data di emissione, scadenza e appuntamento sono incluse nel dataset.
- L'utente può modificare e rifiutare la proposta e vedere che cosa cambierà prima del commit.

### 5.2 WF-02 — Domanda sullo stato personale/familiare

**Esempio:** “Cosa ho domani?”, “Chi porta Leo a calcio?”, “Quali documenti scadono questo mese?”

#### Pipeline

1. normalizzare richiesta e timezone/locale;
2. identificare cerchia e contesto dell'utente;
3. tradurre la richiesta in query tipizzata o piano di retrieval;
4. applicare policy al recupero dati, prima di esporre righe o snippet;
5. eseguire query deterministiche sul modello strutturato;
6. recuperare fonti necessarie e autorizzate;
7. generare una risposta breve con riferimenti alle fonti;
8. validare che ogni affermazione sostanziale sia sostenuta dai dati recuperati;
9. se l'informazione manca o è ambigua, chiedere chiarimento o dire che non è disponibile;
10. registrare modello, fonti referenziate, decisione di policy e esito senza copiare il prompt in analytics.

**Importante:** per domande fattuali sui dati strutturati, query deterministiche hanno precedenza sul retrieval semantico. Il modello può formulare la risposta, ma non deve inventare un evento mancante o colmare un buco con conoscenza generale.

#### Acceptance candidate

- Dataset iniziale: almeno 200 domande etichettate, includendo ambiguità e domande fuori perimetro.
- Ogni risposta contenente un fatto personale ha fonte verificabile; in caso contrario il sistema dichiara l'incertezza.
- Zero rivelazioni in test di query cross-circle e di inferenza su esistenza di oggetti nascosti.
- Separare precisione fattuale, copertura e comportamento di astensione; non usare un singolo punteggio aggregato.

### 5.3 WF-03 — Agente operativo con approvazione e riconciliazione

**Esempio:** creare task, modificare un impegno interno, inviare un invito o fare un'operazione verso un sistema esterno.

#### State machine

`requested → context_resolved → plan_drafted → schema_validated → policy_evaluated → denied | approval_required | approved → execution_started → succeeded | failed | outcome_unknown → reconciled → completed | compensated`

#### Contratto dell'azione

```text
ActionProposal {
  action_id, requester, target_circle, action_type,
  target_resource_refs, proposed_payload, source_refs,
  policy_context_hash, risk_class, reversibility,
  idempotency_key, expires_at, approval_requirement
}
```

Il `policy_context_hash` non sostituisce il ricalcolo della policy. Serve a legare la proposta al contesto che è stato valutato. Prima dell'esecuzione si verifica di nuovo che permessi, target, payload e consenso siano ancora validi.

#### Regole inderogabili

- Il modello propone; il backend decide se l'azione è autorizzata.
- Ogni cambiamento del payload o del destinatario invalida l'approvazione precedente.
- L'approvazione scade e non vale per un'altra azione simile.
- Il retry non deve duplicare pagamenti, invii o creazioni di eventi.
- Se l'API esterna va in timeout dopo aver accettato la richiesta, lo stato è `outcome_unknown`, non `failed`; prima si interroga o riconcilia il sistema remoto.
- Il workflow dichiara se un'azione è annullabile, compensabile o irreversibile.
- La UI mostra gli effetti effettivi, i dati trasferiti e l'eventuale provider esterno.

#### Acceptance candidate

- Il 100% delle tool call mutanti registrate ha una policy decision e un execution id.
- Zero esecuzioni non autorizzate nel corpus di attacchi definito; qualunque caso critico blocca il gate.
- Test di doppio click, retry, timeout, replay, token revocato, cambio permessi durante l'approvazione e modifica del destinatario.

### 5.4 WF-04 — Offline, sincronizzazione e risoluzione conflitti

**Scenario minimo:** due dispositivi di membri autorizzati modificano una lista o un task offline; uno viene riavviato; la rete torna in sequenza diversa; una terza modifica arriva dopo; il sistema deve convergere preservando ogni intenzione o esplicitando un conflitto.

Il workflow deve trattare separatamente:

- stato persistito localmente;
- aggiornamenti inviati/in attesa;
- ack del relay;
- aggiornamenti ricevuti;
- operazioni duplicate;
- conflitti semantici non risolvibili automaticamente;
- invalidazione delle proiezioni di ricerca;
- modifiche dei permessi o membership;
- condizioni di revoca mentre un client è offline.

**Non assumere che tutti i campi siano CRDT naturali.** Un set di checkbox, un testo collaborativo, un intervallo calendario e una transizione di approvazione hanno semantiche diverse. Le regole di merge devono essere specifiche del tipo di dato e includere casi in cui è preferibile mostrare un conflitto anziché scegliere arbitrariamente.

### 5.5 WF-05 — Promemoria e attività schedulate

Il reminder engine deve distinguere calcolo locale, decisione di invio e consegna al canale. L'esito “richiesta accettata dal provider push” non è uguale a “notifica vista dalla persona”.

#### Stati

`scheduled → due → policy_checked → delivery_requested → accepted_by_channel | retryable_failure | permanent_failure → acknowledged | expired | escalated`

Ogni reminder ha: scadenza, timezone, idempotency key, budget interruzioni, importanza, motivazione, fonte, preferenze utente e fallback ammessi. Cambi di timezone, routine editate, disattivazione del consenso o eventi cancellati devono aggiornare il job. Le notifiche critiche necessitano di un canale e un contratto di consegna verificabili; non si deve promettere una percentuale end-to-end senza misurare i provider e i device reali.

### 5.6 WF-06 — Invito, uscita di un membro e ciclo delle chiavi

Questo workflow attraversa identità, membership, policy, cifratura, sync e audit. Va specificato prima di supportare una cerchia con dati riservati.

- Un invito ha ambito, scadenza, ruolo proposto e canale di accettazione.
- Accettazione e autenticazione del dispositivo sono distinte.
- Aggiunta membro → definizione dei dati storici condivisibili → aggiornamento membership e chiavi → sync dei soli oggetti autorizzati.
- Uscita o rimozione → negare accessi futuri → aggiornare membership e chiavi → invalidare sessioni e token → notificare i dispositivi online → gestire client offline secondo minacce e limiti documentati.
- Audit deve registrare l'evento senza rendere visibili contenuti segreti al resto della cerchia.

La revoca di un membro è un caso di test end-to-end, non soltanto una chiamata a `delete member`.

### 5.7 WF-07 — Ingestione connettore esterno

`consent_given → oauth_authorized → initial_sync → mapping_validated → incremental_sync → changes_detected → policy_filter → local_or_server_projection_updated → audit_checkpoint`

Il workflow deve essere idempotente, paginato e riprendibile; deve indicare l'ultimo checkpoint e distinguere errore temporaneo da token revocato o autorizzazione cambiata. Per contenuti provenienti da email o calendari, il contenuto è non attendibile e non può comandare l'agente. Le credenziali non entrano mai nel prompt o negli oggetti di dominio.

### 5.8 WF-08 — Errore AI e fallback

Il sistema deve distinguere almeno: modello non disponibile, timeout, output non conforme allo schema, citazioni non valide, classificazione incerta, policy deny, provider incompatibile con la sensibilità, quota o budget esaurito, offline e versione del modello cambiata.

Fallback accettabili: ripetere con limite e idempotenza; usare un adapter alternativo autorizzato; ridurre la funzione a regole deterministiche; chiedere chiarimento; lasciare una bozza manualmente correggibile; dichiarare il non completamento. È inaccettabile presentare come riuscita una funzione che ha saltato validazione o non ha salvato il risultato.

---

## 6. Perimetro tecnico dell'MVP iniziale

L'MVP è un **vertical slice reale della piattaforma**, non la nuova definizione dell'intero prodotto. Mantiene l'ambizione della visione e dimostra che una traiettoria integrata può funzionare. I moduli non inclusi nella prima prova restano nel piano target e non vengono cancellati dal documento di visione.

### 6.1 Capacità minime da implementare end-to-end

1. **Identità e cerchie (M7):** almeno due membri, ruoli distinti, invito/uscita e membership verificabile.
2. **Confidenzialità e audit (M8):** etichette applicate agli oggetti, policy centrale, traccia di accesso e test negativi tra cerchie.
3. **Grafo e provenienza (M12):** persona, cerchia, fonte, evento/task/documento e relazioni versionate; export/import verificato.
4. **Cattura (M15):** ingresso da testo/share sheet e da almeno un tipo di documento; estrazione proposta e revisione.
5. **Agente (M9):** Q&A basato su dati reali della cerchia, risposta citata e una classe di azione mutante in ambiente controllato con approvazione.
6. **Offline-first (M11):** consultazione e modifica offline di oggetti selezionati, sincronizzazione e test di crash/concorrenza.
7. **Workflow e reminder (M2/M4):** creare un task o impegno da una cattura, approvare una proposta, schedulare un reminder e gestire retry/duplicati.
8. **Routing AI (M11):** almeno un runtime locale e un runtime remoto, con regole esplicite di dati ammessi e fallback; oppure, se il runtime locale non è supportato su un device testato, percorso deterministico offline dimostrato e limite dichiarato.
9. **Superficie mobile:** una piattaforma principale per lo sviluppo iniziale e una seconda piattaforma per dimostrare il core condiviso. La selezione della prima va fatta in base ai dispositivi realmente disponibili ai partecipanti; non si deve dichiarare supporto multipiattaforma prima di avere una build testata.

### 6.2 Workflow dimostrativo principale

**Input:** un utente condivide una circolare/immagine o scrive un impegno.  
**Elaborazione:** Regìa estrae titolo, data, eventuale scadenza e persona coinvolta, legando ciascun campo alla fonte.  
**Revisione:** l'utente vede la proposta, corregge un campo e conferma.  
**Modello dati:** viene creato l'oggetto interno con cerchia, etichetta, provenance e stato.  
**Sync:** il secondo membro autorizzato riceve la modifica; l'oggetto privato non appare in altre cerchie.  
**Agente:** l'utente chiede “cosa devo fare prima?”; il sistema consulta i dati autorizzati, risponde con fonti e può proporre un task.  
**Workflow:** il task si crea soltanto dopo validazione, policy e conferma richiesta.  
**Reminder:** il reminder viene programmato; un test interrompe il backend e il sistema riprende senza duplicare.  
**Audit:** l'utente può vedere fonte, proposta, approvazione e risultato.  
**Offline:** uno dei client modifica il task in modalità aereo; al ritorno in rete il cambiamento converge o viene mostrato come conflitto.  
**Security test:** si ripete con dati segreti, membro rimosso, contenuto con prompt injection e accesso non autorizzato.

Questa dimostrazione è più informativa di dieci funzionalità isolate perché valida l'interazione reale tra le fondamenta.

### 6.3 Fuori dal primo vertical slice, ma non fuori dalla visione

La prima iterazione può rinviare: pagamenti, open banking, telefonate agentiche, automazione PA, assistenza sanitaria funzionale, funzioni ambient avanzate, agenti specialistici multipli, A2A tra agenti, ricerca semantica su grandi volumi e tutte le integrazioni commerciali/esterne non necessarie alla prova. Le relative decisioni di architettura devono però essere registrate come dipendenze future e non chiuse sulla base di assunzioni non testate.

---

## 7. Programma di esperimenti tecnici

Ogni esperimento deve produrre un artefatto ripetibile: codice, dataset/fixture, istruzioni di esecuzione, misure grezze, risultati, difetti aperti, decisione e ADR. “Funziona sul mio dispositivo” non è sufficiente come evidenza.

### 7.1 Work package

| WP | Obiettivo | Attività principali | Deliverable | Criterio di uscita |
|---|---|---|---|---|
| WP0 — Requisiti tecnici e misure | Trasformare la visione in test verificabili | Inventario requisiti, risk register, threat model iniziale, matrice dispositivi/servizi | Requisiti tecnici tracciati e dataset plan | Ogni requisito prioritario ha metodo di verifica e owner |
| WP1 — Dominio e provenienza | Validare grafo, schema ed evoluzione | Tipi, relazioni, sorgenti, stati di conferma, migrazioni, export/import | Schema v0 e suite contratto | Nessuna ambiguità su fonte di verità e ownership dei dati |
| WP2 — Offline/sync | Selezionare modello dati e protocollo | Benchmark Automerge vs alternativa; conflitti, delete, crash, query, migration | Benchmark replicabile e ADR DAT/SYNC | Gate di consistenza, sicurezza e recovery superato |
| WP3 — Policy e membership | Verificare confini tra cerchie | Cedar/OPA spike, casi di accesso, invite/revoke, audit, key lifecycle | Policy catalog, test matrix, threat model aggiornato | Zero bypass critici noti; decisioni e failure modes documentati |
| WP4 — AI gateway/evaluation | Confrontare runtime su task reali | Dataset golden, local/cloud adapters, JSON schema, retrieval e citazioni | Harness eval, risultati per task/provider | Scelta per task supportata da misure; provider non vincolato nel dominio |
| WP5 — Workflow durevoli | Verificare azioni, retry e approval | State machine/outbox vs Temporal, idempotenza, timeout, unknown outcome | Runtime scelto e playbook recovery | Nessun duplicate side effect nei test previsti |
| WP6 — Media/cattura | Dimostrare input→fonte→oggetto | Share, OCR, estrazione, correzione, commit, export e delete | Pipeline multimodale v0 | Fonte e stato confermato preservati; errori correggibili |
| WP7 — Vertical slice | Integrare i componenti | WF-01, WF-02, WF-03 e reminder su dati reali di test | App utilizzabile in ambiente ristretto | Scenario completo ripetibile e nessun gate critico aperto |
| WP8 — Hardening e prove reali | Verificare stabilità e usabilità | Chaos tests, security review, accessibilità, test con partecipanti | Release candidate sperimentale e report | Criteri d'uscita concordati e rischio residuo esplicito |

### 7.2 Sequenza e dipendenze

La sequenza consigliata è **WP0 → WP1 in parallelo a spike WP2/WP3/WP4 → WP5 → WP6 → WP7 → WP8**. Alcuni spike possono procedere in parallelo, ma non si dovrebbe consolidare il backend completo prima delle prove su sync, policy e workflow.

Le integrazioni esterne vanno aggiunte solo dopo che i contratti interni e il comportamento di error recovery sono definiti. La prima integrazione reale dovrà essere di sola lettura o comunque a basso rischio, con account di test e scope minimi.

### 7.3 Scheda obbligatoria di ogni esperimento

```text
ID / versione / owner:
Domanda tecnica da risolvere:
Requisito della Product Vision collegato:
Ipotesi iniziale e alternative:
Threat model rilevante:
Setup riproducibile (OS, device, versioni, modello, dataset):
Metodo e input di test:
Metriche raccolte e unità:
Risultato grezzo e ripetizioni:
Casi falliti / limiti / anomalie:
Effetto sui requisiti e sulle altre componenti:
Decisione: adopt / trial / defer / reject:
ADR collegato:
Condizioni che riaprirebbero la decisione:
```

### 7.4 Comparazione con punteggio

Un punteggio ponderato è utile per confrontare alternative che superano prima i requisiti minimi. Non deve permettere a una grande performance media di compensare un difetto di sicurezza.

| Criterio | Peso proposto | Tipo di verifica |
|---|---:|---|
| Sicurezza, privacy e isolamento | 20% | Gate preliminare + test negativi |
| Correttezza funzionale e preservazione delle intenzioni | 15% | Dataset etichettato e test end-to-end |
| Offline, convergenza e recovery | 15% | Chaos/property tests |
| Qualità AI per task e affidabilità delle citazioni | 10% | Eval separata per famiglia di task |
| Latenza end-to-end | 10% | Misure p50/p95 su device/regioni definiti |
| Operatività e diagnosi dei guasti | 10% | Failure injection, runbook, trace |
| Portabilità e lock-in | 5% | Contratti, export, possibilità di cambiare provider |
| Interoperabilità multipiattaforma | 5% | Build e test su matrice reale |
| Risorse e costo per workflow riuscito | 5% | CPU/GPU, memoria, batteria, costo d'inferenza |
| Semplicità di evoluzione/manutenzione | 5% | ADR, dipendenze, migrazioni, complessità reale |
| **Totale** | **100%** | La sicurezza resta gate, non media compensabile |

I pesi sono una proposta iniziale per la valutazione; possono essere aggiornati prima di eseguire il benchmark, mai dopo aver visto i risultati per favorire un'alternativa.

---

## 8. Piano di test e criteri di accettazione

### 8.1 Livelli di test

- **Unit test:** invarianti di dominio, normalizzazioni, policy helpers, transizioni di stato, serializzazione.
- **Property-based/fuzz test:** parsing, merge, schema validation, invarianti di oggetti, input malformati e messaggi sync.
- **Contract test:** API interne, connettori, model adapters, schema MCP e persistenza.
- **Integration test:** database, workflow, provider simulati, chiavi, migrazioni e proiezioni.
- **End-to-end:** workflow completo dal gesto dell'utente all'audit e alla visibilità del risultato.
- **Chaos/failure injection:** crash, perdita rete, reorder dei messaggi, timeout, duplicate delivery, token scaduto, provider fuori servizio e ripristino da backup.
- **Security test:** isolamento cerchie, revoca, accesso a oggetti nascosti, escalation privilegi, prompt injection, tool injection e leakage through metadata.
- **Usabilità/accessibilità:** task osservati con persone reali, differenti capacità digitali e dispositivi; evitare che modalità semplificate aggirino informazioni importanti o controlli di consenso.
- **Evaluation AI:** dataset versionati, test di regressione per modello, provider, prompt, router e pipeline di retrieval.

### 8.2 Gate non negoziabili

Prima di consentire uso con dati personali reali su più persone:

1. Nessuna vulnerabilità critica aperta nel perimetro sottoposto a security review.
2. Nessuna azione mutante può eseguirsi senza una decisione di policy valida, controllata fuori dal modello.
3. Nessun contenuto etichettato come non autorizzato può essere recuperato via query, ricerca, snippet, prompt, notifica, export o sync nei test avversariali previsti.
4. Nessuna duplicazione di side effect nei casi di retry definiti; gli outcome incerti devono essere riconciliati.
5. Export, restore e migrazione devono essere esercitati su dati non fittizi ma controllati, verificando integrità e cancellazione.
6. Devono esistere una modalità di revoca accessi e una procedura d'incidente comprensibili.
7. Il trattamento dei dati e gli accessi devono essere spiegati ai partecipanti; niente raccolta opaca di contenuti per debug.

### 8.3 Soglie quantitative iniziali

Le soglie seguenti sono **proposte iniziali di engineering gate**. Vanno confermate dopo il primo baseline test; non sono una garanzia di prestazione né devono diventare una scusa per ignorare errori critici.

| Area | Soglia iniziale | Note |
|---|---|---|
| Domande su dati strutturati | ≥95% corrette su benchmark etichettato; 100% dei fatti personali citabili a una fonte | Misurare separatamente precisione, copertura e astensione |
| Estrazione di evento/scadenza | ≥90% campi essenziali corretti nel corpus iniziale | Data tipo (emissione/scadenza/evento) valutata separatamente |
| Source provenance | 100% degli oggetti creati dal flusso primario hanno fonte oppure motivo dichiarato | Campo obbligatorio e test automatico |
| Authorization enforcement | 100% tool calls mutanti con decisione e audit; zero bypass critici | Gate, non media statistica |
| Convergenza | 100% degli scenari testati converge o presenta conflitto esplicito senza perdita silenziosa | Esplicitare copertura degli scenari; non promettere per ogni sequenza possibile senza prova formale |
| Recovery workflow | 100% dei fault injectati nel corpus esce in stato coerente e osservabile | Verificare side effect esterno, non solo DB interno |
| Latenza interazioni locali | p95 <100 ms per azioni locali definite come “semplici”, su device baseline | Misurare end-to-end dalla UI alla persistenza |
| Latenza voce | p50 e p95 per primo audio misurati; usare <800 ms / <1.500 ms come obiettivi da validare | Specificare rete, codec, device e definizione di “primo audio” |
| Export | Export completo verificato e round-trip su dataset di riferimento; tempo misurato su volume dichiarato | Non fissare un tempo senza dimensione/dati e hardware di riferimento |
| Batteria | Consumo misurato su test standard di 24 h; target <3%/giorno solo per task background specifici | Non attribuire al sistema intero senza definire baseline e attività |
| Prompt injection | Zero azioni non autorizzate nella suite critica; aumentare corpus oltre i casi elementari | Il numero di test è copertura minima, non prova universale di sicurezza |
| Compatibilità device | Matrice versioni/OS/supporto AI pubblicata; fallback deterministico dimostrato | Mai dichiarare “on-device” in generale quando vale solo per alcuni dispositivi |

### 8.4 Metriche AI da non confondere

- **Accuracy/precision/recall**: correttezza rispetto al gold set; cambiano per task e classe.
- **Groundedness**: affermazioni sostenute dalle fonti recuperate.
- **Coverage**: casi in cui il sistema fornisce una risposta invece di astenersi.
- **Appropriate abstention**: rifiuto/clarificazione quando dati o autorizzazioni sono insufficienti.
- **Action safety**: assenza di esecuzioni non autorizzate, duplicazioni o destinatari errati.
- **Latency**: misurata end-to-end, inclusi retrieval, policy, rete e serializzazione.
- **Cost/resource per success**: risorse divise per workflow completati correttamente, non per chiamata API isolata.
- **Regression stability**: variazione dopo update di model, runtime, prompt o schema.

Un sistema che risponde a più domande ma inventa fonti o rivela dati non autorizzati non è migliore. I gate di sicurezza restano prioritari.

---

## 9. Qualità del software e processo di delivery

### 9.1 Repository e struttura

Possibile struttura iniziale, da adattare dopo lo spike sui binding e runtime:

```text
/regia
  /apps
    /ios
    /android
    /web-lab (eventuale)
  /core
    /domain
    /policy-adapter
    /provenance
    /action-contracts
    /sync-adapter
    /crypto-adapter
  /backend
    /api
    /workflow
    /connectors
    /ai-gateway
  /schemas
    /api
    /events
    /actions
    /policy-fixtures
  /evals
    /datasets (senza dati personali reali non autorizzati)
    /harness
    /reports
  /infra
    /dev
    /test
    /prod-like
  /docs
    /adr
    /threat-model
    /runbooks
    /experiments
```

Evitare un unico repository pieno di dipendenze senza ownership. Se un monorepo migliora la sincronizzazione delle versioni e i contratti, può essere usato, purché siano chiari i confini di build e i test per modulo.

### 9.2 Definition of Done per una capacità

Una funzione non è “fatta” perché il happy path funziona. La Definition of Done richiede:

- requisito e criterio di accettazione tracciati;
- modello dati e policy aggiornati;
- test unitari, integrazione ed end-to-end appropriati;
- retry, timeout e casi di errore specificati;
- trattamento dei dati e privacy documentati;
- telemetria tecnica redatta;
- audit per le azioni pertinenti;
- gestione migrazioni, export e cancellazione ove applicabili;
- accessibilità e copy della UI verificati;
- limiti noti e fallback dichiarati;
- documentazione di supporto e runbook se il sistema dipende da un servizio operativo;
- benchmark/eval AI aggiornati se la funzione utilizza modelli.

### 9.3 CI/CD e ambienti

La pipeline dovrebbe eseguire almeno:

- formatting, lint, type-check e build riproducibile;
- unit/contract tests;
- test su migrazioni e schema compatibility;
- suite di policy e test negativi;
- regressione AI su dataset versionati, con rilevazione del drift;
- dependency/license scan, secrets scan e static analysis;
- SBOM per build condivise con i partecipanti;
- packaging e firma degli artefatti;
- integration tests in ambiente isolato con provider simulati;
- release di prova canary e possibilità di rollback della versione app/backend, senza assumere rollback dei dati già migrati.

I prompt, gli schemi, le policy, i modelli e i connettori sono asset versionati. Aggiornare un prompt o un modello deve essere trattato come un cambiamento software con test, versione e possibilità di disattivazione.

### 9.4 Gestione della configurazione

Configurare per ambiente: provider consentiti per ciascuna classe di dati, modelli/versioni ammessi, budget/limiti, connettori abilitati, retention, modalità debug, politiche di approvazione e flag sperimentali. Le configurazioni sicure sono il default. Non lasciare che un feature flag abiliti una funzione ad alto impatto senza lo stesso enforcement di policy.

### 9.5 Gestione delle versioni AI

Ogni risultato AI deve poter essere ricondotto a task type, provider, model identifier/versione nota, adapter version, schema output, prompt/template version, parametri rilevanti e data di esecuzione. Se il provider cambia modello senza offrire una versione stabile, la regressione deve essere rilevata attraverso le eval programmate e la funzione deve avere un fallback o un kill switch.

---

## 10. Pianificazione, ownership e governance tecnica

Il ruolo di project manager è assicurare che ogni work package abbia obiettivo, output, criteri di accettazione, owner e dipendenze; il principal engineer assicura coerenza del sistema, evidenze per le scelte, qualità dei contratti e gestione dei rischi tecnici. Se una sola persona svolge entrambi i ruoli, le responsabilità restano distinte per evitare che l'urgenza di implementare sostituisca la valutazione tecnica.

### 10.1 Ownership suggerita

| Area | Accountable | Responsabilità principali |
|---|---|---|
| Visione e priorità dei flussi | Product/Experiment Owner | Sceglie i bisogni reali da validare, conferma i criteri di utilità e partecipa ai gate |
| Architettura e ADR | Principal Engineer | Confini, decisioni, trade-off, contratti e revisione architetturale |
| Dati/sync | Tech lead domain/sync | Schema, CRDT/SQL comparison, migrazioni, conflitti e recovery |
| Security/privacy | Security owner (anche consulente esterno) | Threat model, key lifecycle, access tests, incident response e revisione indipendente |
| AI/evaluation | AI engineer/owner | Dataset, harness, provider adapters, quality/safety evals e fallback |
| Workflow/connettori | Backend engineer | State machine, idempotenza, side effects, rate limit, reconciliation |
| Mobile/accessibilità | Client engineer | Superfici, lifecycle, offline UX, accessibilità e integrazioni OS |
| Pilot participants | Facilitator/owner | Consenso informato, feedback, onboarding, supporto e uscita dal test |

### 10.2 Riunioni e artefatti decisionali

- **Architecture review:** ad ogni scelta che impatta dati, sicurezza, provider, sync o contratti pubblici.
- **Experiment review:** al termine di ogni spike, con risultati riproducibili e una decisione esplicita.
- **Risk review:** ogni volta che si abilita una categoria dati o una nuova azione esterna.
- **Pilot review:** feedback settimanale o per ciclo di prova, con difetti distinti per gravità e impatto.

Artefatti minimi: ADR, risk register, decision log, data flow map, threat model, schema/contratti, benchmark report, release notes, known limitations e runbook di recovery.

### 10.3 Architecture Decision Records

Ogni ADR deve contenere: contesto, vincoli, alternative, decisione, prove consultate, conseguenze positive e negative, rischi residui, migrazione/rollback, owner, data, stato (`proposed`, `accepted`, `superseded`, `rejected`) e condizioni di revisione.

Le decisioni da aprire subito sono:

- ADR-001: perimetro del core Rust;
- ADR-002: modello dati e source of truth;
- ADR-003: motore di sync e semantiche di conflitto;
- ADR-004: key lifecycle, E2EE e MLS;
- ADR-005: motore di policy Cedar vs OPA/Rego;
- ADR-006: modular monolith, storage e backend API;
- ADR-007: state machine/outbox vs Temporal;
- ADR-008: interfaccia AI gateway e task router;
- ADR-009: provider/runtime on-device e cloud;
- ADR-010: telemetria e audit;
- ADR-011: strategia di ricerca e indici;
- ADR-012: client nativi vs cross-platform.

---

## 11. Rischi principali, indicatori e mitigazioni

| ID | Rischio | Probabilità iniziale | Impatto | Indicatore precoce | Mitigazione / decisione |
|---|---|---|---|---|---|
| R-01 | CRDT + SQL + E2EE genera una complessità non sostenibile | Alta | Critico | Indici divergenti, merge anomali, recovery manuale | Spike prima del consolidamento; una sola fonte di verità per ogni oggetto |
| R-02 | La rimozione di un membro non copre device offline e copie storiche | Media-alta | Critico | Accesso persistente dopo revoca; ambiguità sui dati storici | Threat model e test CRY; definire esattamente il perimetro della revoca |
| R-03 | Le policy vengono applicate in modo diverso da agent, UI, sync e ricerca | Alta | Critico | Un endpoint o snippet non rispetta il test delle cerchie | Unico contract policy; test negativi end-to-end; deny-by-default |
| R-04 | LLM inventa o trasforma un dato estratto in fatto | Alta | Alto | Scadenze/partecipanti senza fonte | Provenance per campo, validazioni deterministic, review per campi critici |
| R-05 | Prompt injection provoca un'azione esterna | Media-alta | Critico | Tool call derivata da email/PDF non verificato | Data/instruction separation, scope minimi, approvazione, red-team e no direct model execution |
| R-06 | Retry duplica un invio o una mutazione remota | Media | Alto | Duplicati dopo timeout | Idempotency key, outbox, outcome_unknown e reconciliation |
| R-07 | Provider AI cambia qualità o comportamento | Alta | Medio-alto | Regressione in eval o aumento astensioni | Versioning, eval periodiche, fallback e routing multi-provider |
| R-08 | Supporto on-device frammentato | Alta | Alto | Funzione disponibile soltanto su pochi device | Device matrix reale, adapter e fallback dichiarato |
| R-09 | Modello E2EE impedisce capacità di query/AI previste | Alta | Alto | Richieste cloud richiedono plaintext non autorizzato | Data-flow mapping per task; minimizzazione e consenso specifico |
| R-10 | MVP tenta di integrare tutti i moduli prima di validare le fondamenta | Alta | Alto | Molti moduli incompleti, nessun end-to-end affidabile | Un solo vertical slice reale, architettura target mantenuta nel design |
| R-11 | Telemetria contiene dati personali | Media | Alto | Prompt o titoli nei log/traces | Redaction by default, test di log, access control e retention breve |
| R-12 | Rischi legali/consensi sottovalutati nei pilot | Media | Alto | Dati di minori/salute condivisi senza processo adeguato | Pilot protocol, minimizzazione, informativa e verifica professionale quando necessario |
| R-13 | SLO e soglie della visione sono irrealistici/non misurati | Media-alta | Medio-alto | Non c'è baseline o definizione metrica | Formalizzare measurement protocol e ricalibrare prima di dichiarare supporto |
| R-14 | Il grafo diventa uno schema universale impossibile da evolvere | Media | Alto | Troppi tipi generici, query e validazioni speciali | Bounded contexts, tipi espliciti, schema versionato e casi d'uso reali |
| R-15 | Integrazioni esterne non ufficiali o scope troppo ampi | Media | Alto | Token con permessi non necessari, API fragili | Canali ufficiali, scope minimi, sandbox, scadenze e review delle condizioni |

### 11.1 Rischio residuo e uso con persone reali

Il passaggio da dati sintetici a dati reali deve essere un gate esplicito. Si comincia con dataset sintetici o anonimizzati, poi con dati reali dell'ideatore, quindi con una cerchia limitata di partecipanti informati. Ogni espansione a minori, salute, finanza o altri dati ad alto impatto richiede una review mirata e non è automaticamente autorizzata dal superamento del test tecnico generale.

L'esperimento non elimina gli obblighi legali applicabili. Prima di trattare dati di altre persone occorre valutare finalità, ruoli, base giuridica, informativa, diritti, retention, eventuali dati particolari e necessità di una valutazione d'impatto. La relazione è una specifica d'ingegneria, non un parere legale.

---

## 12. Mappatura dei moduli M1–M17 alle capacità di piattaforma

| Moduli visione | Capacità di piattaforma necessarie | Dipendenze da validare |
|---|---|---|
| M1 Calendario | Modello temporale, timezone, connettori, conflict detection, policy | Fonte canonica, sync, semantica eventi esterni e recurring rules |
| M2 Task/progetti | Domain model, dipendenze, assegnazioni, workflow, audit | Merge concurrente, autorizzazioni, notifiche |
| M3 Routine | Scheduler, pattern statistici, preferenze e spiegabilità | Dati temporali sufficienti, false positives, opt-out |
| M4 Reminder | Scheduler durevole, push/SMS provider, budget e escalation | Consegna vs visualizzazione, timezone, duplicate delivery |
| M5 Spese/finanze | Documento/OCR, categorie, importi, connector/banking | Data governance, accuratezza importi, canali regolamentati |
| M6 Spesa/dispensa | Lista collaborativa, prodotti, voice/capture, sync | Merge delle checkbox, duplicate item e offline |
| M7 Cerchie/ruoli | Identity, membership, policy, invitation, lifecycle chiavi | Ruoli asimmetrici, famiglia ricomposta, revoke behavior |
| M8 Privacy/confidenzialità | Policy, encryption, audit, deletion, data classification | Metadata leakage, index, model routing, key recovery |
| M9 Agente | AI gateway, retrieval, tool registry, policy, workflow | Prompt injection, idempotenza, astensione e approval |
| M10 Voce | STT/TTS, streaming, locale, UI/accessibility | Latenza end-to-end, background, consenso e provider terms |
| M11 Offline/LLM locale | Local DB, sync, runtime adapters, router | Device support, battery, exact privacy behavior |
| M12 Second brain | Grafo, provenance, search, vectors optional, citations | Indici rigenerabili e retrieval authorization |
| M13 Burocrazia | OCR, templates con fonte, deadline workflow, document vault | Fonti aggiornate, validazione delle date e retention |
| M14 Cura/salute | Sensitivity tags, reminders, sharing asymmetry, audit | Dati particolari, accesso caregiver, limiti clinici e normativi |
| M15 Cattura universale | Share sheet, email ingestion, OCR, media pipeline | Input non attendibili, attachment security, consent scopes |
| M16 Viaggi | Calendar, location, API esterne, planning e notifications | Current external data, permission location, booking side effects |
| M17 Ambient/superfici | Shared application APIs, device presence, permissions | Privacy in spazi condivisi e rivelazione involontaria |

Questa tabella aiuta a evitare che ogni modulo si costruisca una propria versione di sync, permessi, AI routing o audit. Le capacità trasversali devono essere piattaforma; i moduli devono esporre contratti di dominio e workflow specifici.

---

## 13. Roadmap tecnica con gate decisionali

La roadmap è definita per **risultati tecnici**, non per date promesse. Le durate saranno pianificate dopo aver verificato team, device, competenze e infrastruttura disponibili.

### Gate G0 — Baseline verificabile

**Output:** requisiti tecnici tracciati, threat model iniziale, schema delle entità principali, benchmark plan, policy preliminari e ADR aperti.  
**Non si procede se:** proprietà di source of truth, classificazione dati, cerchie e definizione degli scenari di rischio sono ambigue.

### Gate G1 — Fondamenta comparate

**Output:** decisioni supportate da test su storage/sync, policy engine, key management, client strategy e workflow runtime.  
**Non si procede se:** il sistema non può spiegare chi vede cosa, come riprende un workflow e come evita duplicazioni.

### Gate G2 — Vertical slice integrato

**Output:** flusso completo cattura → provenienza → revisione → oggetto → query citata → proposta di azione → approvazione → reminder → audit, con offline e cerchie.  
**Non si procede se:** rimangono bypass critici, perdite silenziose, false dichiarazioni di successo o nessun recovery testato.

### Gate G3 — Pilot ristretto

**Output:** rilascio controllato su dispositivi e scenari dichiarati, supporto e procedura d'uscita; misure di utilità e difetti osservati.  
**Non si procede se:** partecipanti non comprendono dati trattati/accessi/limiti o non c'è modo affidabile di revocare e recuperare.

### Gate G4 — Estensione progressiva

**Output:** aggiunta dei moduli e connettori successivi sulla base delle piattaforme validate; benchmark e security regressions continuano.  
**Condizione:** ogni estensione introduce il proprio test di rischio e non aggira i controlli della piattaforma.

La roadmap non limita la visione: impedisce che la profondità della visione venga scambiata per prova che tutte le dipendenze siano già risolte.

---

## 14. Decisioni preliminari e open questions

### 14.1 Decisioni che possono essere prese subito

- L'agente non è un'autorità di autorizzazione e non esegue direttamente side effect.
- La provenienza è un dato di prima classe, non un testo decorativo allegato dopo.
- Le policy sono valutate da codice deterministico fuori dal modello.
- L'MVP va valutato end-to-end e deve preservare i confini architetturali della visione.
- I componenti AI, di storage, sync e workflow sono sostituibili dietro contratti quando ciò riduce il rischio; non si costruisce un framework astratto per ogni futura possibilità.
- Tutti i dati derivati (embedding, snippet, cache, allegati temporanei) hanno ciclo di vita e policy, non sono scarti invisibili.
- Le performance e le soglie numeriche devono essere confermate con un protocollo di misura.

### 14.2 Decisioni da non fissare prima degli spike

- Automerge come fonte di verità definitiva per ogni oggetto;
- usare CRDT per tutte le entità, a prescindere dalla semantica;
- MLS e una specifica libreria di implementazione in produzione;
- Cedar o OPA/Rego senza confronto con le policy di Regìa;
- Temporal senza misurare il carico operativo rispetto alla state machine;
- provider cloud primario o modello locale standard per tutti i dispositivi;
- indice vettoriale e motore dedicato prima che le query SQL/FTS abbiano benchmark;
- framework agentico multi-agent prima di validare un solo workflow governato;
- MCP/A2A come framework interno per tutto il dominio;
- SLO di disponibilità e latenza prima di definire infrastruttura e measurement boundary.

### 14.3 Domande che richiedono risposta da owner/engineering

1. Quali dispositivi iOS/Android/desktop sono effettivamente disponibili per il primo esperimento e quali versioni OS vanno supportate?
2. Quale modello di account/identity è realistico nel contesto iniziale (email, passkey, login provider, recovery)?
3. Il requisito di E2EE si applica a tutti gli oggetti condivisi o prima di tutto a un sottoinsieme con etichette particolarmente sensibili? La policy deve essere esplicita, non implicita.
4. Quali dati devono restare inaccessibili anche al backend operatore, e quali workflow richiedono invece elaborazione server-side?
5. Qual è la semantica attesa quando un membro revocato torna online con dati modificati in precedenza?
6. Quali sistemi reali (calendar, email, documenti) saranno usati nel primo vertical slice e quali hanno accesso API ufficiale disponibile?
7. Quanti partecipanti e dispositivi concorrenti sono realisticamente previsti nel test iniziale?
8. Dove viene eseguito il backend di prova e chi può accedere ai log amministrativi?
9. Quali categorie di dati sono escluse dal primo pilot finché non sono state completate le verifiche specifiche?
10. Quali obiettivi di latenza e disponibilità sono determinanti per il primo caso d'uso e come saranno misurati?

Se alcune risposte sono ancora sconosciute, il lavoro può iniziare con la matrice di test e un ambiente sintetico; non è necessario inventare assunzioni per chiudere le decisioni premature.

---

## 15. Conclusione tecnica

La migliore architettura per Regìa non è quella che contiene il maggior numero di componenti moderni, né quella che massimizza la quota di codice scritta in un unico linguaggio. È quella che riesce a soddisfare con evidenze i requisiti più caratterizzanti: confini di accesso affidabili, funzionamento offline utile, sincronizzazione coerente, provenienza della conoscenza, azioni governate, workflow recuperabili e AI sostituibile.

La proposta è quindi di procedere con un **programma di prove comparate in cui le prime decisioni sono dati/sync/E2EE, policy e workflow**, mentre client e AI vengono valutati su un vertical slice reale. Si raccomanda una base mobile nativa, un core Rust selettivo, storage locale SQL come candidato e Automerge/MLS/Cedar-OPA/Temporal come scelte da validare nei rispettivi spike. Il backend parte modulare; AI e connettori restano dietro contratti; ogni azione mutante viene governata da policy, approvazione quando richiesta, idempotenza e audit.

L'MVP deve dimostrare la qualità delle fondamenta e un workflow completo davvero utile. La visione completa resta ampia: le funzionalità non implementate non vengono espunte, ma dipendono da decisioni e prove esplicite. Questo è il modo per costruire un esperimento tecnologico ambizioso senza trasformare l'ambizione in una somma di assunzioni non testate.

---

## 16. Fonti tecniche e riferimenti

**Data di verifica delle fonti web:** 20 ottobre 2026. Le pagine ufficiali sono riferimenti di documentazione e stato dei protocolli/framework, non prove indipendenti della superiorità di un prodotto nello scenario specifico di Regìa. L'idoneità finale va verificata in test.

### 16.1 Riferimenti di Regìa

- **[RV]** *Regìa — Product Vision & Technical Experiment Charter v2.0*, documento di progetto, 20 ottobre 2026. Fonte dei requisiti funzionali M1–M17, A1–A15, principi, modello dati e ipotesi architetturali.

### 16.2 Runtime e architettura client

- **[RUST]** Rust, Release Notes — release 1.99.1 (15 ottobre 2026): https://doc.rust-lang.org/releases.html
- **[TAURI]** Tauri v2, Architecture: https://v2.tauri.app/concept/architecture/ (alternativa cross-platform da confrontare, non scelta obbligata)
- **[APPLE-FM]** Apple Developer Documentation, Foundation Models e aggiornamenti 2026: https://developer.apple.com/documentation/foundationmodels e https://developer.apple.com/documentation/updates/foundationmodels
- **[ANDROID-GENAI]** Google ML Kit GenAI APIs: https://developers.google.com/ml-kit/genai
- **[LITERT]** Google Developers Blog, LiteRT: The Universal Framework for On-Device AI (28 gennaio 2026): https://developers.googleblog.com/litert-the-universal-framework-for-on-device-ai/
- **[ONNX-MOBILE]** ONNX Runtime, mobile application development: https://onnxruntime.ai/docs/tutorials/mobile/

### 16.3 Storage, sincronizzazione e crittografia

- **[AUTOMERGE-CONCEPTS]** Automerge Docs, Concepts and Sync Protocol: https://automerge.org/docs/reference/concepts/
- **[AUTOMERGE-REPO]** Automerge Docs, Repositories: https://automerge.org/docs/reference/repositories/
- **[AUTOMERGE-STORAGE]** Automerge Docs, Storage: https://automerge.org/docs/reference/repositories/storage/
- **[MLS]** IETF RFC 9420, The Messaging Layer Security (MLS) Protocol: https://www.rfc-editor.org/rfc/rfc9420.html
- **[SQLITE-PRAGMA]** SQLite, PRAGMA statements e comportamento WAL/synchronous: https://www.sqlite.org/pragma.html
- **[SQLITE-FTS5]** SQLite, FTS5 Full-Text Search: https://www.sqlite.org/fts5.html

### 16.4 Policy, workflow e interoperabilità

- **[CEDAR]** Cedar Policy Language Reference Guide: https://docs.cedarpolicy.com/
- **[OPA]** Open Policy Agent, documentazione e linguaggio Rego: https://www.openpolicyagent.org/docs/
- **[TEMPORAL]** Temporal, durable execution e AI workflows: https://docs.temporal.io/temporal e https://docs.temporal.io/ai
- **[MCP-SPEC]** Model Context Protocol, specifica del 28 luglio 2026: https://modelcontextprotocol.io/specification/2026-07-28
- **[MCP-AUTH]** Model Context Protocol, autorizzazione e sicurezza: https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/authorization
- **[A2A]** Agent2Agent Protocol, documentazione ufficiale: https://a2a-protocol.org/v1.0.0/

### 16.5 Sicurezza, rischio AI e osservabilità

- **[OWASP-LLM]** OWASP GenAI Security Project, Top 10 for LLM Applications 2025: https://genai.owasp.org/llm-top-10/
- **[OWASP-ASVS]** OWASP Application Security Verification Standard 5.0.0: https://asvs.owasp.org/
- **[NIST-AI-RMF]** NIST, AI Risk Management Framework e aggiornamenti: https://www.nist.gov/itl/ai-risk-management-framework
- **[NIST-GENAI]** NIST AI 600-1, Generative AI Profile: https://doi.org/10.6028/NIST.AI.600-1
- **[NIST-PRIVACY]** NIST Privacy Framework: https://www.nist.gov/privacy-framework
- **[OTEL]** OpenTelemetry Semantic Conventions: https://opentelemetry.io/docs/specs/semconv/

### 16.6 Come interpretare le fonti

- La documentazione ufficiale è la fonte preferita per funzionalità dichiarate, specifiche e requisiti dei protocolli; non dimostra che una tecnologia sia la scelta ottimale per Regìa.
- L'uscita di una nuova API o di un modello non implica copertura dei dispositivi, stabilità di API, qualità linguistica italiana o adeguatezza per dati sensibili.
- Le prestazioni dichiarate da un provider vanno misurate insieme agli altri stadi del workflow e ai casi reali dell'esperimento.
- Qualunque affermazione normativa deve essere riverificata rispetto al testo in vigore e al contesto specifico prima di usarla come decisione di compliance.
- Le scelte di stack contenute in questa relazione sono un giudizio ingegneristico preliminare da convalidare; non sono una certificazione o una garanzia.

---

## Appendice A — Template per Architecture Decision Record

```markdown
# ADR-NNN — Titolo

- Stato: proposed | accepted | superseded | rejected
- Data:
- Owner:
- Requisiti di riferimento:

## Contesto
Quale problema concreto di Regìa deve essere risolto? Quali vincoli non sono negoziabili?

## Alternative considerate
A. ...
B. ...
C. ...

## Metodo di valutazione
Dataset, ambiente, versioni, misure, threat model, numero di ripetizioni.

## Decisione
Scelta e condizioni di utilizzo.

## Evidenze
Risultati misurati, regressioni, failure modes e limiti.

## Conseguenze
Benefici, compromessi, debito tecnico e conseguenze su altri moduli.

## Migrazione / rollback
Come cambiare scelta senza perdita di dati o senza aprire accessi.

## Rischi residui
Cosa non è stato verificato.

## Condizioni per riaprire la decisione
Quali fatti nuovi o metriche richiedono revisione?
```

## Appendice B — Checklist release sperimentale

- [ ] Requisiti e casi d'uso della release tracciati
- [ ] Threat model aggiornato per le nuove funzioni
- [ ] Policy e test negativi superati
- [ ] Migration/restore/export verificati
- [ ] Test offline, retry e duplicate side effect superati
- [ ] Dataset/eval AI eseguiti e regressioni esaminate
- [ ] Versioni di modelli, prompt, schema e policy registrate
- [ ] Scope OAuth e segreti verificati
- [ ] Audit e telemetria senza payload personale non necessario
- [ ] Disclosures, permessi e consensi UX verificati
- [ ] Limitazioni note e fallback visibili
- [ ] Runbook e rollback pronti
- [ ] Partecipanti informati e modalità di uscita disponibili
- [ ] Rischi residui accettati dall'owner per questa specifica fase
