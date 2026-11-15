//! R0: deterministic rules for Italian and English. The baseline every model must beat, and the offline path.
//!
//! Conservative by construction: when the text is ambiguous or impossible it asks (`Clarify`); when it does not
//! understand it proposes nothing. It never follows text inside brackets or an imperative aimed at the assistant.
// ponytail: hand-written token rules, enough for the golden phrases. Upgrade path: a real date grammar (e.g. a
// small parser-combinator) only if R0 is kept as a product path after AI-01.

// Token-index arithmetic reads best with short names (a, b, i, n, h, m); the module is a throwaway baseline.
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

use time::{Date, Duration, Month, OffsetDateTime};

use crate::contract::{Adapter, Dated, Obj, ObjKind, Outcome, Proposal, TaskRequest};
use regia_domain::DateRole;

/// The rules adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct R0;

impl Adapter for R0 {
    fn id(&self) -> &'static str {
        "r0"
    }
    fn version(&self) -> &'static str {
        "r0-rules-1"
    }
    fn local(&self) -> bool {
        true
    }
    fn extract(&self, req: &TaskRequest) -> Result<Proposal, String> {
        Ok(extract(req))
    }
}

struct Tok {
    s: usize,
    e: usize,
    w: String,
}

const MONTHS: [&str; 12] = [
    "gennaio january",
    "febbraio february",
    "marzo march",
    "aprile april",
    "maggio may",
    "giugno june",
    "luglio july",
    "agosto august",
    "settembre september",
    "ottobre october",
    "novembre november",
    "dicembre december",
];
const WEEKDAYS: [&str; 7] = [
    "lunedì lunedi monday",
    "martedì martedi tuesday",
    "mercoledì mercoledi wednesday",
    "giovedì giovedi thursday",
    "venerdì venerdi friday",
    "sabato saturday",
    "domenica sunday",
];
const EVENT_CUES: [&str; 14] = [
    "riunione",
    "cena",
    "visita",
    "appuntamento",
    "colloquio",
    "partita",
    "aperitivo",
    "compleanno",
    "meeting",
    "dinner",
    "appointment",
    "match",
    "offsite",
    "festa",
];
const DOC_CUES: [&str; 9] = [
    "bolletta",
    "fattura",
    "verbale",
    "tassa",
    "bill",
    "invoice",
    "notice",
    "pagamento",
    "payment",
];
const TASK_VERBS: [&str; 8] = [
    "chiama",
    "chiamare",
    "compra",
    "comprare",
    "call",
    "buy",
    "ricordami",
    "avvisami",
];
const SKIP_FIRST: [&str; 14] = [
    "ignora",
    "ignore",
    "elimina",
    "delete",
    "cancella",
    "condividi",
    "share",
    "rivela",
    "reveal",
    "print",
    "segna",
    "mark",
    "assistente",
    "assistant",
];

fn in_list(list: &[&str], w: &str) -> bool {
    list.iter().any(|l| l.split(' ').any(|x| x == w))
}

fn index_of(list: &[&str], w: &str) -> Option<usize> {
    list.iter().position(|l| l.split(' ').any(|x| x == w))
}

fn strip_brackets(t: &str) -> String {
    let mut depth = 0_u32;
    let mut out = String::with_capacity(t.len());
    for c in t.chars() {
        let blank = depth > 0 || c == '[';
        match c {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if blank || c == ']' {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        } else {
            out.push(c);
        }
    }
    out
}

fn tokens(t: &str) -> Vec<Tok> {
    let cs: Vec<(usize, char)> = t.char_indices().collect();
    let word = |c: char| c.is_alphanumeric() || matches!(c, '\'' | '’' | ':' | '/');
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if !word(cs[i].1) {
            i += 1;
            continue;
        }
        let st = i;
        while i < cs.len() {
            let c = cs[i].1;
            let in_number = matches!(c, '.' | ',')
                && i > st
                && cs[i - 1].1.is_ascii_digit()
                && cs.get(i + 1).is_some_and(|n| n.1.is_ascii_digit());
            if word(c) || in_number {
                i += 1;
            } else {
                break;
            }
        }
        let s = cs[st].0;
        let end = cs.get(i).map_or(t.len(), |x| x.0);
        let raw = t[s..end].trim_end_matches(':');
        if !raw.is_empty() {
            out.push(Tok {
                s,
                e: s + raw.len(),
                w: raw.to_lowercase(),
            });
        }
    }
    out
}

fn month_of(w: &str) -> Option<Month> {
    let n = index_of(&MONTHS, w)?;
    Month::try_from(u8::try_from(n + 1).ok()?).ok()
}

fn digits(w: &str) -> Option<u32> {
    (!w.is_empty() && w.bytes().all(|b| b.is_ascii_digit()))
        .then(|| w.parse().ok())
        .flatten()
}

/// `(hour, minute, meridiem)` of `7`, `17:30`, `3pm`, `10:15am`. `meridiem`: `Some(true)` pm, `Some(false)` am.
fn clock(w: &str) -> Option<(u32, u32, Option<bool>)> {
    let (body, mer) = if let Some(b) = w.strip_suffix("pm") {
        (b, Some(true))
    } else if let Some(b) = w.strip_suffix("am") {
        (b, Some(false))
    } else {
        (w, None)
    };
    match body.split_once(':') {
        Some((h, m)) => Some((digits(h)?, digits(m)?, mer)),
        None => (w.len() <= 4).then_some((digits(body)?, 0, mer)),
    }
}

/// Parses a decimal amount in Italian or English notation into `1200.00`; `None` if malformed.
fn amount(w: &str) -> Option<String> {
    let last = w.rfind([',', '.']);
    let (int, dec) = match last {
        Some(p) if w.len() - p - 1 == 2 => (&w[..p], &w[p + 1..]),
        _ => (w, "00"),
    };
    let groups: Vec<&str> = int.split([',', '.']).collect();
    let ok = groups[0].len() <= 3 && !groups[0].is_empty() || groups.len() == 1;
    let grouped = groups.iter().skip(1).all(|g| g.len() == 3);
    let all_digits = groups.iter().all(|g| g.bytes().all(|b| b.is_ascii_digit()));
    (ok && grouped && all_digits && dec.bytes().all(|b| b.is_ascii_digit()))
        .then(|| format!("{}.{dec}", groups.concat()))
}

fn numlike(w: &str) -> bool {
    w.bytes().next().is_some_and(|b| b.is_ascii_digit())
        && w.bytes()
            .all(|b| b.is_ascii_digit() || b == b'.' || b == b',')
}

fn is_day_word(w: &str) -> bool {
    matches!(w, "oggi" | "domani" | "dopodomani" | "today" | "tomorrow") || in_list(&WEEKDAYS, w)
}

/// Returns a clarification reason if anything in the text is impossible or ambiguous.
fn needs_clarification(tk: &[Tok], now: OffsetDateTime) -> Option<String> {
    let w = |i: usize| tk.get(i).map_or("", |t| t.w.as_str());
    for (i, k) in tk.iter().enumerate() {
        let next_is_month = month_of(w(i + 1)).is_some();
        if (k.w.contains(':') || k.w.ends_with("pm") || k.w.ends_with("am"))
            && let Some((h, m, _)) = clock(&k.w)
            && (h >= 24 || m >= 60)
        {
            return Some("impossible time".into());
        }
        if k.w.contains('/') {
            let p: Vec<Option<u32>> = k.w.split('/').map(digits).collect();
            if let [Some(d), Some(m), Some(y)] = p[..] {
                let ok = u8::try_from(m)
                    .ok()
                    .and_then(|m| Month::try_from(m).ok())
                    .is_some_and(|m| {
                        u8::try_from(d).is_ok_and(|d| {
                            i32::try_from(y)
                                .is_ok_and(|y| Date::from_calendar_date(y, m, d).is_ok())
                        })
                    });
                if !ok {
                    return Some("impossible date".into());
                }
            }
        }
        if let (Some(d), true) = (digits(&k.w), next_is_month) {
            let y = digits(w(i + 2))
                .filter(|y| *y > 999)
                .and_then(|y| i32::try_from(y).ok());
            let m = month_of(w(i + 1));
            let ok = m.zip(u8::try_from(d).ok()).is_some_and(|(m, d)| {
                Date::from_calendar_date(y.unwrap_or_else(|| now.year()), m, d).is_ok()
            });
            if !ok {
                return Some("impossible date".into());
            }
        }
        if numlike(&k.w)
            && (w(i + 1) == "euro" || (i > 0 && w(i - 1) == "euro"))
            && amount(&k.w).is_none()
        {
            return Some("malformed amount".into());
        }
        let ordinal =
            k.w.strip_suffix("th")
                .or(k.w.strip_suffix("st"))
                .or(k.w.strip_suffix("nd"))
                .or(k.w.strip_suffix("rd"));
        let bare_il = k.w == "il"
            && digits(w(i + 1)).is_some_and(|d| (1..=31).contains(&d))
            && month_of(w(i + 2)).is_none();
        if bare_il || (ordinal.and_then(digits).is_some() && !next_is_month) {
            return Some("day of the month without a month".into());
        }
        if is_day_word(&k.w)
            && matches!(w(i + 1), "o" | "or")
            && (is_day_word(w(i + 2)) || (w(i + 2) == "the" && w(i + 3) == "day"))
        {
            return Some("two candidate days".into());
        }
        if in_list(&WEEKDAYS, &k.w) && i > 0 && matches!(w(i - 1), "next" | "prossimo") {
            return Some("next weekday is ambiguous".into());
        }
        if matches!(k.w.as_str(), "alle" | "at")
            && let Some((h, _, None)) = clock(w(i + 1))
            && (1..=6).contains(&h)
            && !(2..=3).any(|d| {
                matches!(
                    w(i + d),
                    "mattina"
                        | "pomeriggio"
                        | "sera"
                        | "notte"
                        | "morning"
                        | "afternoon"
                        | "evening"
                        | "night"
                )
            })
        {
            return Some("am or pm".into());
        }
        let pair = (k.w.as_str(), w(i + 1));
        if matches!(
            pair,
            ("qualche", "giorni") | ("few", "days") | ("usual", "time") | ("solito", "orario")
        ) {
            return Some("no resolvable date".into());
        }
    }
    None
}

enum When {
    Day(Date),
    Hours(i64),
}

fn word_number(w: &str) -> Option<i64> {
    match w {
        "un" | "uno" | "una" | "one" | "an" | "a" => Some(1),
        "due" | "two" => Some(2),
        "tre" | "three" => Some(3),
        "quattro" | "four" => Some(4),
        "cinque" | "five" => Some(5),
        _ => digits(w).and_then(|d| i64::from(d).into()),
    }
}

fn last_day_of_month(d: Date) -> Date {
    let first_next = if d.month() == Month::December {
        Date::from_calendar_date(d.year() + 1, Month::January, 1)
    } else {
        Date::from_calendar_date(d.year(), d.month().next(), 1)
    };
    first_next.map_or(d, |f| f - Duration::days(1))
}

/// A date or relative-time phrase starting at token `i`: `(when, tokens consumed)`.
fn when_at(tk: &[Tok], i: usize, now: OffsetDateTime) -> Option<(When, usize)> {
    let today = now.date();
    let w = |j: usize| tk.get(j).map_or("", |t| t.w.as_str());
    let day = |n: i64| When::Day(today + Duration::days(n));
    match w(i) {
        "oggi" | "today" => return Some((day(0), 1)),
        "domani" | "tomorrow" => return Some((day(1), 1)),
        "dopodomani" => return Some((day(2), 1)),
        "the" if (w(i + 1), w(i + 2), w(i + 3)) == ("day", "after", "tomorrow") => {
            return Some((day(2), 4));
        }
        "day" if (w(i + 1), w(i + 2)) == ("after", "tomorrow") => return Some((day(2), 3)),
        "fine" if w(i + 1) == "mese" => return Some((When::Day(last_day_of_month(today)), 2)),
        "end" if (w(i + 1), w(i + 2), w(i + 3)) == ("of", "the", "month") => {
            return Some((When::Day(last_day_of_month(today)), 4));
        }
        "un'ora" => return None,
        _ => {}
    }
    if let Some(target) = index_of(&WEEKDAYS, w(i)) {
        let cur = i64::from(today.weekday().number_days_from_monday());
        let target = i64::try_from(target).ok()?;
        return Some(if matches!(w(i + 1), "prossimo" | "prossima") {
            (day(7 - cur + target), 2)
        } else {
            let ahead = (target - cur).rem_euclid(7);
            (day(if ahead == 0 { 7 } else { ahead }), 1)
        });
    }
    if matches!(w(i), "tra" | "in") {
        if w(i + 1) == "un'ora" {
            return Some((When::Hours(1), 2));
        }
        let n = word_number(w(i + 1))?;
        return match w(i + 2) {
            "giorni" | "giorno" | "days" | "day" => Some((day(n), 3)),
            "settimane" | "settimana" | "weeks" | "week" => Some((day(n * 7), 3)),
            "ora" | "ore" | "hour" | "hours" => Some((When::Hours(n), 3)),
            _ => None,
        };
    }
    explicit_date(tk, i, now).map(|(d, n)| (When::Day(d), n))
}

fn explicit_date(tk: &[Tok], i: usize, now: OffsetDateTime) -> Option<(Date, usize)> {
    let w = |j: usize| tk.get(j).map_or("", |t| t.w.as_str());
    if w(i).contains('/') {
        let p: Vec<u32> = w(i).split('/').filter_map(digits).collect();
        if let [d, m, y] = p[..] {
            let date = Date::from_calendar_date(
                i32::try_from(y).ok()?,
                Month::try_from(u8::try_from(m).ok()?).ok()?,
                u8::try_from(d).ok()?,
            )
            .ok()?;
            return Some((date, 1));
        }
        return None;
    }
    let d = u8::try_from(digits(w(i))?).ok()?;
    let m = month_of(w(i + 1))?;
    let year = digits(w(i + 2))
        .filter(|y| *y > 999)
        .and_then(|y| i32::try_from(y).ok());
    let used = if year.is_some() { 3 } else { 2 };
    let mut date = Date::from_calendar_date(year.unwrap_or_else(|| now.year()), m, d).ok()?;
    if year.is_none() && date < now.date() {
        date = Date::from_calendar_date(now.year() + 1, m, d).ok()?;
    }
    Some((date, used))
}

struct Temporal {
    when: When,
    time: Option<(u32, u32)>,
    first: usize,
    last: usize,
}

fn find_temporal(
    tk: &[Tok],
    a: usize,
    b: usize,
    now: OffsetDateTime,
) -> Result<Option<Temporal>, String> {
    let Some((i, when, n)) = (a..b).find_map(|i| when_at(tk, i, now).map(|(w, n)| (i, w, n)))
    else {
        return Ok(None);
    };
    let (mut first, mut last) = (i, i + n);
    let mut time = None;
    for j in a..b {
        if !matches!(tk[j].w.as_str(), "alle" | "at") || j + 1 >= b {
            continue;
        }
        let w = tk[j + 1].w.as_str();
        time = if matches!(w, "noon" | "mezzogiorno") {
            Some((12, 0))
        } else {
            clock(w).and_then(|(h, m, mer)| match mer {
                Some(true) if h < 12 => Some((h + 12, m)),
                Some(false) if h == 12 => Some((0, m)),
                _ if h < 24 && m < 60 => Some((h, m)),
                _ => None,
            })
        };
        if time.is_some() {
            first = first.min(j);
            last = last.max(j + 2);
        }
    }
    if matches!(when, When::Day(_)) || time.is_none() {
        return Ok(Some(Temporal {
            when,
            time,
            first,
            last,
        }));
    }
    Err("time without a day".into())
}

fn hhmm(t: (u32, u32)) -> String {
    format!("{:02}:{:02}", t.0, t.1)
}

fn instant(at: OffsetDateTime) -> String {
    let (h, m, _) = at.offset().as_hms();
    let sign = if h < 0 || m < 0 { '-' } else { '+' };
    format!(
        "{}T{:02}:{:02}:00{sign}{:02}:{:02}",
        at.date(),
        at.hour(),
        at.minute(),
        h.abs(),
        m.abs()
    )
}

fn clean_title(raw: &str) -> String {
    let s = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut s = s.trim_matches(|c: char| !c.is_alphanumeric()).to_owned();
    for p in [
        "ricordami di ",
        "remind me to ",
        "alert me to ",
        "devo ",
        "i have to ",
        "nota ",
        "appunto ",
        "note ",
        "idea ",
    ] {
        if s.to_lowercase().starts_with(p) {
            s = s[p.len()..].to_owned();
            break;
        }
    }
    for tail in [
        " entro", " by", " per", " on", " il", " in", " di", " alle", " at", " e", " and",
    ] {
        if s.to_lowercase().ends_with(tail) {
            s.truncate(s.len() - tail.len());
        }
    }
    let mut c = s.chars();
    c.next().map_or(s.clone(), |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    })
}

fn dated(role: DateRole, d: Date, time: Option<(u32, u32)>) -> Dated {
    Dated {
        role,
        value: d.to_string(),
        time: time.map(hhmm),
        evidence: None,
    }
}

fn clause(
    t: &str,
    tk: &[Tok],
    a: usize,
    b: usize,
    now: OffsetDateTime,
) -> Result<Option<Obj>, String> {
    let words = &tk[a..b];
    let first = words[0].w.as_str();
    let has = |x: &str| words.iter().any(|k| k.w == x);
    if in_list(&SKIP_FIRST, first)
        || (has("system") && has("prompt"))
        || has("istruzioni")
        || has("instructions")
    {
        return Ok(None);
    }
    let ctext = &t[words[0].s..words[b - a - 1].e];
    let tm = find_temporal(tk, a, b, now)?;
    let second = words.get(1).map_or("", |k| k.w.as_str());
    let note_cue = matches!(first, "nota" | "appunto" | "idea" | "note")
        || (first, second) == ("ricorda", "che")
        || (first, second) == ("remember", "that");
    let mk = |kind, title: String, evidence: &str| Obj {
        kind,
        title,
        evidence: evidence.to_owned(),
        date: None,
        dates: vec![],
        amount_eur: None,
        remind_at: None,
    };
    let Some(tm) = tm else {
        return Ok(if note_cue {
            Some(mk(ObjKind::Note, clean_title(ctext), ctext))
        } else if in_list(&TASK_VERBS, first) {
            Some(mk(ObjKind::Task, clean_title(ctext), ctext))
        } else {
            None
        });
    };
    let ev = &t[tk[tm.first].s..tk[tm.last - 1].e];
    let title = clean_title(&ctext.replacen(ev, "", 1));
    let deadline = tm.first > a && matches!(tk[tm.first - 1].w.as_str(), "entro" | "by");
    let mut o = mk(ObjKind::Task, title, ev);
    let has_event_cue = words.iter().any(|k| in_list(&EVENT_CUES, &k.w));
    let remind_word = matches!(first, "avvisami" | "alert");
    match tm.when {
        When::Hours(n) => o.remind_at = Some(instant(now + Duration::hours(n))),
        When::Day(d) if remind_word && tm.time.is_some() => {
            let (h, m) = tm.time.unwrap_or_default();
            let at = d
                .with_hms(
                    u8::try_from(h).unwrap_or(0),
                    u8::try_from(m).unwrap_or(0),
                    0,
                )
                .map_err(|e| e.to_string())?;
            o.remind_at = Some(instant(at.assume_offset(now.offset())));
        }
        When::Day(d) if has_event_cue && !deadline => {
            o.kind = ObjKind::Event;
            o.date = Some(dated(DateRole::Appointment, d, tm.time));
        }
        When::Day(d) => o.date = Some(dated(DateRole::Due, d, tm.time)),
    }
    Ok(Some(o))
}

fn doc_role(tk: &[Tok], from: usize, to: usize) -> Option<DateRole> {
    if to > 0 && tk[to - 1].w == "del" {
        return Some(DateRole::Issued);
    }
    tk[from..to].iter().rev().find_map(|k| match k.w.as_str() {
        "emessa" | "emesso" | "emissione" | "issued" | "dated" => Some(DateRole::Issued),
        "scadenza" | "scade" | "scadeva" | "entro" | "due" | "by" => Some(DateRole::Due),
        "pagata" | "pagato" | "pagamento" | "paid" | "payment" => Some(DateRole::Payment),
        _ => None,
    })
}

/// A document: a cue word plus at least one explicit date. `None` if this is not a document.
fn document(t: &str, tk: &[Tok], now: OffsetDateTime) -> Option<Result<Obj, String>> {
    let cue = tk.iter().position(|k| {
        DOC_CUES.contains(&k.w.as_str()) || (k.w == "fine" && tk.iter().any(|x| x.w == "issued"))
    })?;
    let mut dates = Vec::new();
    let mut prev_end = 0;
    let mut i = 0;
    while i < tk.len() {
        if let Some((d, n)) = explicit_date(tk, i, now) {
            let Some(role) = doc_role(tk, prev_end, i) else {
                return Some(Err("cannot tell which date this is".into()));
            };
            let ev = t[tk[i].s..tk[i + n - 1].e].to_owned();
            dates.push(Dated {
                role,
                value: d.to_string(),
                time: None,
                evidence: Some(ev),
            });
            prev_end = i + n;
            i += n;
        } else {
            i += 1;
        }
    }
    if dates.is_empty() {
        return None;
    }
    let amount_eur = tk.iter().enumerate().find_map(|(j, k)| {
        let near_euro =
            tk.get(j + 1).is_some_and(|n| n.w == "euro") || (j > 0 && tk[j - 1].w == "euro");
        (numlike(&k.w) && near_euro).then(|| amount(&k.w)).flatten()
    });
    let title: String = t.split(',').next().unwrap_or(t).chars().take(60).collect();
    Some(Ok(Obj {
        kind: ObjKind::Document,
        title: clean_title(&title),
        evidence: t[tk[cue].s..tk[cue].e].to_owned(),
        date: None,
        dates,
        amount_eur,
        remind_at: None,
    }))
}

/// Token ranges of the clauses of the text; separators and connectives are dropped.
fn clauses(clean: &str, tk: &[Tok]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 1;
    while i < tk.len() {
        let gap = &clean[tk[i - 1].e..tk[i].s];
        let w = tk[i].w.as_str();
        let next = tk.get(i + 1).map_or("", |k| k.w.as_str());
        let skip = if gap.contains(['.', '!', '?', '\n']) {
            Some(0)
        } else if matches!((w, next), ("e", "poi") | ("and", "then")) {
            Some(2)
        } else if gap.contains(',') && matches!(w, "e" | "and") {
            Some(1)
        } else {
            None
        };
        if let Some(n) = skip {
            if i > start {
                out.push((start, i));
            }
            start = i + n;
            i += n.max(1);
        } else {
            i += 1;
        }
    }
    if start < tk.len() {
        out.push((start, tk.len()));
    }
    out
}

fn clarify(why: &str) -> Proposal {
    Proposal {
        outcome: Outcome::Clarify,
        objects: vec![],
        clarify_reason: Some(why.into()),
    }
}

fn run(req: &TaskRequest) -> Result<Proposal, String> {
    let t = req.input.as_str();
    let clean = strip_brackets(t);
    let tk = tokens(&clean);
    let nothing = Proposal {
        outcome: Outcome::Nothing,
        objects: vec![],
        clarify_reason: None,
    };
    if tk.is_empty() || t.contains('?') {
        return Ok(nothing);
    }
    if tk.len() == 1 {
        return Err("a single word: task, event or note?".into());
    }
    if let Some(why) = needs_clarification(&tk, req.now) {
        return Err(why);
    }
    let mut objects = Vec::new();
    if let Some(doc) = document(t, &tk, req.now) {
        objects.push(doc?);
    } else {
        for (a, b) in clauses(&clean, &tk) {
            objects.extend(clause(t, &tk, a, b, req.now)?);
        }
    }
    Ok(if objects.is_empty() {
        nothing
    } else {
        Proposal {
            outcome: Outcome::Objects,
            objects,
            clarify_reason: None,
        }
    })
}

fn extract(req: &TaskRequest) -> Proposal {
    run(req).unwrap_or_else(|why| clarify(&why))
}
