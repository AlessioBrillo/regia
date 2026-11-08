//! Contract suite for the logical entity schema v0.
//!
//! `fixtures/v0/valid/*.json` are the wire format: each must parse and serialise back to the
//! same JSON value, so any change to the format fails here and forces a conscious decision.
//! `fixtures/v0/invalid/*.json` must each be refused, for the reason listed below.
//! Fixtures are synthetic; never add real personal data.

use std::{collections::BTreeSet, fs, path::PathBuf};

use regia_domain::Envelope;
use serde_json::Value;

fn dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0")
        .join(name)
}

fn files(name: &str) -> Vec<(String, String)> {
    let mut out: Vec<_> = fs::read_dir(dir(name))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
            (stem, fs::read_to_string(&p).unwrap())
        })
        .collect();
    out.sort();
    out
}

#[test]
fn valid_fixtures_roundtrip_exactly() {
    let mut types = BTreeSet::new();
    for (name, text) in files("valid") {
        let env: Envelope = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let original: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(serde_json::to_value(&env).unwrap(), original, "{name}");
        types.insert(original["object"]["type"].as_str().unwrap().to_owned());
    }
    // Every entity of the schema is covered by at least one valid fixture.
    let expected: BTreeSet<String> = [
        "circle",
        "membership",
        "person",
        "source",
        "task",
        "event",
        "document",
    ]
    .map(str::to_owned)
    .into();
    assert_eq!(types, expected);
}

/// Fixture name → text the refusal must contain. Every invalid fixture must be listed.
const REFUSED: &[(&str, &str)] = &[
    ("blank_circle_name", "must not be blank"),
    ("blank_unsourced_reason", "non-blank reason"),
    ("circle_own_parent", "own parent"),
    ("confidence_over_1000", "0..=1000"),
    ("event_ends_before_start", "ends before it starts"),
    ("extra_envelope_field", "unknown field `trace`"),
    ("extra_field", "unknown field `revision`"),
    ("future_schema_version", "unsupported schema version 1"),
    ("id_not_uuid_v7", "UUIDv7"),
    ("missing_provenance", "missing field `provenance`"),
    (
        "missing_server_visibility",
        "missing field `server_visibility`",
    ),
    ("sourced_without_source", "missing field `first`"),
    ("unknown_date_role", "unknown variant `birthday`"),
    ("unknown_label", "unknown variant `top_secret`"),
    ("unknown_type", "unknown variant `payment`"),
    ("unknown_visibility", "unknown variant `plaintext`"),
];

#[test]
fn invalid_fixtures_are_refused_for_the_stated_reason() {
    let found: BTreeSet<_> = files("invalid").into_iter().map(|(n, _)| n).collect();
    let listed: BTreeSet<_> = REFUSED.iter().map(|(n, _)| (*n).to_owned()).collect();
    assert_eq!(found, listed, "fixture files and REFUSED table must match");

    for (name, text) in files("invalid") {
        let reason = REFUSED.iter().find(|(n, _)| *n == name).unwrap().1;
        let err = serde_json::from_str::<Envelope>(&text)
            .expect_err(&format!("{name} must be refused"))
            .to_string();
        assert!(err.contains(reason), "{name}: {err:?} lacks {reason:?}");
    }
}
