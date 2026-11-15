//! Routing cases R-01..R-08 (docs/experiments/AI-01). No network: adapters are fakes or R0.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Fake, nothing, req};
use regia_domain::Confidentiality;
use regia_spike_ai01::{Gateway, Refusal, Validation, r0::R0};

fn cloud() -> Fake {
    Fake {
        id: "cloud",
        local: false,
        out: Ok(nothing()),
    }
}

fn local_failing() -> Fake {
    Fake {
        id: "local",
        local: true,
        out: Err("model crashed".into()),
    }
}

/// R-01: `secret` with only cloud adapters stops; it is never routed in the clear.
#[test]
fn r01_secret_with_only_cloud_stops() {
    let mut g = Gateway::default();
    g.add(Box::new(cloud()));
    let (res, audit) = g.run(&req("anything", Confidentiality::Secret), "cloud");
    assert_eq!(res.unwrap_err(), Refusal::LabelForbidsCloud);
    assert!(audit.reason.starts_with("refused"));
}

/// R-02: `restricted`, preferred local runtime unavailable: no automatic cloud fallback.
#[test]
fn r02_restricted_never_falls_back_to_cloud() {
    let mut g = Gateway::default();
    g.add(Box::new(cloud()));
    let (res, _) = g.run(&req("anything", Confidentiality::Restricted), "local");
    assert_eq!(res.unwrap_err(), Refusal::Unavailable);
}

/// R-03: a fallback to another local runtime is allowed, never silent.
#[test]
fn r03_local_fallback_is_recorded() {
    let mut g = Gateway::default();
    g.add(Box::new(cloud()));
    g.add(Box::new(R0));
    let (res, audit) = g.run(&req("Chiama Luca", Confidentiality::Restricted), "local");
    let r = res.unwrap();
    assert_eq!(r.runtime, "r0");
    assert_eq!(r.fallback_reason.as_deref(), Some("local unavailable"));
    assert_eq!(audit.reason, "local unavailable");
}

/// R-04: a canary marker closes the cloud channel even when the label is low and consent was given.
#[test]
fn r04_canary_closes_the_cloud_channel() {
    let mut g = Gateway::default();
    g.add(Box::new(cloud()));
    let (res, _) = g.run(
        &req("nota CANARY-3-7 qualcosa", Confidentiality::Circle),
        "cloud",
    );
    assert_eq!(res.unwrap_err(), Refusal::CanaryForbidsCloud);
}

/// The allowed path exists and needs consent: low label, no canary, explicit consent.
#[test]
fn cloud_runs_only_with_consent() {
    let mut g = Gateway::default();
    g.add(Box::new(cloud()));
    let mut r = req("Buy milk", Confidentiality::Circle);
    r.cloud_consent = false;
    assert_eq!(g.run(&r, "cloud").0.unwrap_err(), Refusal::NeedsConsent);
    r.cloud_consent = true;
    assert!(g.run(&r, "cloud").0.is_ok());
}

/// R-05: the routing decision is a function of metadata only; the input text never reaches it.
#[test]
fn r05_router_sees_metadata_not_text() {
    use regia_spike_ai01::{Meta, route, router::Candidate};
    let a = Meta::of(&req("one text", Confidentiality::Circle));
    let b = Meta::of(&req("a completely different text", Confidentiality::Circle));
    let c = [Candidate {
        id: "cloud".into(),
        local: false,
    }];
    assert_eq!(route(&a, "cloud", &c), route(&b, "cloud", &c));
}

/// R-06: the audit record holds runtime, version, label and reason, and no input.
#[test]
fn r06_audit_has_no_input() {
    let mut g = Gateway::default();
    g.add(Box::new(R0));
    let text = "Nota CANARY-3-7: esito delle analisi";
    let (_, audit) = g.run(&req(text, Confidentiality::Restricted), "r0");
    let shown = format!("{audit:?} {}", serde_json::to_string(&audit).unwrap());
    assert!(!shown.contains("CANARY") && !shown.contains("analisi"));
    assert_eq!(
        (audit.runtime.as_str(), audit.version.as_str()),
        ("r0", "r0-rules-1")
    );
}

/// R-07: a failing runtime is a clear failure, not a false success and not a silent switch to the cloud.
#[test]
fn r07_runtime_failure_is_visible() {
    let mut g = Gateway::default();
    g.add(Box::new(local_failing()));
    g.add(Box::new(cloud()));
    let r = g
        .run(&req("Buy milk", Confidentiality::Circle), "local")
        .0
        .unwrap();
    assert_eq!(r.runtime, "local");
    assert!(r.proposal.is_none());
    assert!(matches!(r.validation, Validation::Failed(ref m) if m.contains("model crashed")));
}

/// R-08: an offline-required task does not use a cloud runtime.
#[test]
fn r08_offline_required_stays_local() {
    let mut g = Gateway::default();
    g.add(Box::new(cloud()));
    let mut r = req("Buy milk", Confidentiality::Circle);
    r.offline_required = true;
    assert_eq!(
        g.run(&r, "cloud").0.unwrap_err(),
        Refusal::OfflineRequiresLocal
    );
}
