# HT-01 — Verifica ambiente (giorno 1)

| Campo | Valore |
|---|---|
| ID | HT-01 |
| Area | setup |
| Tempo massimo | 2 h |
| Stato | pronta |
| Issue | da aprire con template `Human task` |

## Perché serve una persona

L'agente può scrivere le istruzioni, ma non può verificare che firma GPG, hook e Docker funzionino sulle tue macchine.

## Obiettivo

Avere Windows 11 e Arch Linux in grado di eseguire il gate, con commit firmati e indirizzo noreply.

## Input e prerequisiti

- Accesso al repo e chiave GPG o SSH registrata su GitHub
- Indirizzo email noreply di GitHub
- Windows 11 desktop e Arch Linux box
- Docker attivo (serve a `ci-local`)

## Procedura

1. Clona il repo su entrambe le macchine.
2. Configura git con i comandi qui sotto, sostituendo l'indirizzo tra parentesi con il tuo indirizzo noreply.
3. Installa il toolchain: `rust-toolchain.toml` seleziona la versione 1.98.1; verifica con `cargo --version`.
4. Installa [just](https://github.com/casey/just#installation) e verifica `just --version`.
5. Su Arch esegui `just setup` e poi `just gate` (che chiama `ci-local`). Su Windows 11 esegui `ci-local.cmd` da PowerShell.
6. Registra in un commento: versione OS, `rustc -V`, `cargo -V`, `just -V`, `docker -v` ed esito del gate.
7. Fai un commit di prova locale (senza push) e controlla che sia firmato e con `Signed-off-by`.

## Comandi

```sh
git config core.hooksPath .githooks
git config commit.gpgsign true
git config user.email "<il-tuo-indirizzo>@users.noreply.github.com"
git commit -s -m "chore: verify local setup"
git log -1 --show-signature
```

## Evidenze da conservare

- Output del gate, anche se fallisce
- Tabella con versioni e sistemi operativi
- Output di `git log -1 --show-signature`

## Criteri di accettazione

- [ ] `ci-local` esce con codice 0 su almeno una macchina
- [ ] Il commit di prova è firmato e porta `Signed-off-by`
- [ ] Nessuna email non-noreply nei commit nuovi
- [ ] Eventuali problemi segnalati su issue con label `needs-owner`

## Dove finisce il risultato

Commento sull'issue della card. Nessun file nuovo nel repo.

## Se ti blocchi

Commenta con `needs-owner` e allega l'errore esatto. Non passare alle altre card prima di aver chiarito il blocco.
