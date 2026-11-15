//! Deterministic validation after every adapter. A model error caught here is a failure, not a dangerous error.

use time::{
    Date, OffsetDateTime,
    format_description::well_known::{Iso8601, Rfc3339},
};

use crate::contract::{Dated, Obj, ObjKind, Outcome, Proposal, TaskRequest};

fn valid_time(t: &str) -> bool {
    let Some((h, m)) = t.split_once(':') else {
        return false;
    };
    h.len() == 2
        && m.len() == 2
        && matches!((h.parse::<u8>(), m.parse::<u8>()), (Ok(h), Ok(m)) if h < 24 && m < 60)
}

fn valid_amount(a: &str) -> bool {
    a.split_once('.').is_some_and(|(i, d)| {
        !i.is_empty() && d.len() == 2 && i.bytes().chain(d.bytes()).all(|b| b.is_ascii_digit())
    })
}

fn check_dated(req: &TaskRequest, d: &Dated) -> Result<(), String> {
    if d.value.len() != 10 || Date::parse(&d.value, &Iso8601::DATE).is_err() {
        return Err(format!("bad date {:?}", d.value));
    }
    if d.time.as_deref().is_some_and(|t| !valid_time(t)) {
        return Err("bad time".into());
    }
    match &d.evidence {
        Some(e) if !req.input.contains(e.as_str()) => Err("date evidence not in input".into()),
        _ => Ok(()),
    }
}

fn check_obj(req: &TaskRequest, o: &Obj) -> Result<(), String> {
    if o.title.trim().is_empty() {
        return Err("empty title".into());
    }
    if o.evidence.trim().is_empty() || !req.input.contains(o.evidence.as_str()) {
        return Err("evidence not in input".into());
    }
    o.date
        .iter()
        .chain(&o.dates)
        .try_for_each(|d| check_dated(req, d))?;
    if o.amount_eur.as_deref().is_some_and(|a| !valid_amount(a)) {
        return Err("bad amount".into());
    }
    if o.remind_at
        .as_deref()
        .is_some_and(|r| OffsetDateTime::parse(r, &Rfc3339).is_err())
    {
        return Err("bad reminder instant".into());
    }
    if o.kind == ObjKind::Document && o.dates.is_empty() {
        return Err("document without dates".into());
    }
    Ok(())
}

/// Checks schema shape, dates, amounts and that every piece of evidence is verbatim in the input (AR-06).
///
/// # Errors
/// A short reason; the proposal must then be discarded.
pub fn validate(req: &TaskRequest, p: &Proposal) -> Result<(), String> {
    match (p.outcome, p.objects.is_empty()) {
        (Outcome::Objects, true) => return Err("objects outcome without objects".into()),
        (Outcome::Clarify | Outcome::Nothing, false) => {
            return Err("objects on a non-objects outcome".into());
        }
        _ => {}
    }
    p.objects.iter().try_for_each(|o| check_obj(req, o))
}
