//! DAT-01 synthetic dataset generator (WP0 benchmark plan, `docs/experiments/DAT-01-data-model-sync.md`).
//!
//! The output is a pure function of `(seed, profile)`: same input, same bytes. The example
//! `dat01` prints it as one JSON `Envelope` per line; the DAT-01 spike consumes it directly.
//!
//! Everything is synthetic. Objects labelled `restricted` or `secret` carry a `CANARY-<circle>-<n>`
//! marker in their title: leakage tests (AR-04) search indexes, logs, caches and notifications
//! for it, and any hit outside an authorised view is a failure.
//!
//! `ponytail:` the PRNG is `splitmix64`, fine for fixtures and never for anything secret.

use crate::{
    Assertion, Circle, CircleId, Confidence, Confidentiality, DateRole, DatedValue, Document,
    Envelope, Event, Fact, Locator, Membership, Object, ObjectHeader, ObjectId, Person, PersonId,
    PrincipalId, Provenance, Role, ServerVisibility, Source, SourceId, SourceKind, SourceRef, Task,
    TaskStatus, Timestamp, UnsourcedReason,
};
use time::OffsetDateTime;
use uuid::{Builder, Uuid};

/// Cardinalities of a dataset. Declared in the experiment card; keep both in sync.
pub struct Profile {
    /// Profile name (`small` or `large`).
    pub name: &'static str,
    principals: usize,
    extra_persons: usize,
    circles: usize,
    sources_per_circle: usize,
    objects_per_circle: usize,
}

/// 6 circles, 76 lines at seed 42.
pub const SMALL: Profile = Profile {
    name: "small",
    principals: 4,
    extra_persons: 3,
    circles: 6,
    sources_per_circle: 2,
    objects_per_circle: 6,
};

/// 40 circles, about 6 650 lines.
pub const LARGE: Profile = Profile {
    name: "large",
    principals: 50,
    extra_persons: 40,
    circles: 40,
    sources_per_circle: 10,
    objects_per_circle: 150,
};

/// 2026-10-26T00:00:00Z. Fixed so output never depends on the wall clock.
const BASE_SECS: i64 = 1_793_000_000;
const BASE_MS: u64 = 1_793_000_000_000;
const DAY: i64 = 86_400;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`n` is clamped to at least 1).
    fn below(&mut self, n: u32) -> u32 {
        u32::try_from(self.next() % u64::from(n.max(1))).unwrap_or(0)
    }

    fn idx(&mut self, len: usize) -> usize {
        usize::try_from(self.below(u32::try_from(len).unwrap_or(u32::MAX))).unwrap_or(0)
    }

    fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }
}

struct CircleInfo {
    id: CircleId,
    index: usize,
    sources: Vec<(SourceId, OffsetDateTime)>,
    persons: Vec<PersonId>,
}

struct Gen {
    rng: Rng,
    clock: u64,
    out: Vec<Envelope>,
}

impl Gen {
    fn push(&mut self, object: Object) {
        match Envelope::new(object) {
            Ok(e) => self.out.push(e),
            Err(e) => panic!("generator produced an invalid object: {e}"),
        }
    }

    /// A fresh time-ordered `UUIDv7`, deterministic in the seed.
    fn uuid(&mut self) -> Uuid {
        self.clock += 1;
        let mut tail = [0u8; 10];
        for chunk in tail.chunks_mut(8) {
            let bytes = self.rng.next().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        Builder::from_unix_timestamp_millis(BASE_MS + self.clock, &tail).into_uuid()
    }

    fn id<T: TryFrom<Uuid>>(&mut self) -> T {
        let u = self.uuid();
        T::try_from(u).unwrap_or_else(|_| unreachable!("the builder sets version 7"))
    }

    fn instant(&mut self, from_secs: i64, span_days: u32) -> OffsetDateTime {
        let secs = from_secs
            + i64::from(self.rng.below(span_days)) * DAY
            + i64::from(self.rng.below(86_400));
        OffsetDateTime::from_unix_timestamp(secs).unwrap_or_else(|_| unreachable!("in range"))
    }

    fn label(&mut self) -> Confidentiality {
        match self.rng.below(10) {
            0..=1 => Confidentiality::Public,
            2..=5 => Confidentiality::Circle,
            6..=7 => Confidentiality::Restricted,
            _ => Confidentiality::Secret,
        }
    }

    /// `restricted` and `secret` never leave the device readable (report §4.5, label rules).
    fn visibility(&mut self, label: Confidentiality) -> ServerVisibility {
        if label >= Confidentiality::Restricted {
            return ServerVisibility::EndToEndEncrypted;
        }
        match self.rng.below(3) {
            0 => ServerVisibility::ServerReadable,
            1 => ServerVisibility::EndToEndEncrypted,
            _ => ServerVisibility::TransientWorker,
        }
    }

    fn provenance(&mut self, circle: &CircleInfo) -> Provenance {
        if circle.sources.is_empty() || !self.rng.chance(70) {
            let reasons = ["typed by user", "imported test fixture", "derived by rule"];
            let reason = reasons[self.rng.idx(reasons.len())];
            return Provenance::Unsourced {
                reason: UnsourcedReason::new(reason).unwrap_or_else(|_| unreachable!("non-blank")),
            };
        }
        let first = self.source_ref(circle);
        let more = if self.rng.chance(20) {
            vec![self.source_ref(circle)]
        } else {
            vec![]
        };
        Provenance::Sourced { first, more }
    }

    fn source_ref(&mut self, circle: &CircleInfo) -> SourceRef {
        let (source, captured_at) = circle.sources[self.rng.idx(circle.sources.len())];
        let locator = if self.rng.chance(50) {
            Locator::Page(1 + self.rng.below(20))
        } else {
            let start = self.rng.below(500);
            Locator::CharSpan {
                start,
                end: start + 1 + self.rng.below(100),
            }
        };
        SourceRef {
            source,
            locator: Some(locator),
            captured_at,
        }
    }

    fn fact<T>(&mut self, value: T, circle: &CircleInfo) -> Fact<T> {
        let provenance = self.provenance(circle);
        let sourced = matches!(provenance, Provenance::Sourced { .. });
        Fact {
            value,
            assertion: if sourced {
                Assertion::Proposed
            } else {
                Assertion::Confirmed
            },
            confidence: sourced
                .then(|| {
                    Confidence::new(500 + u16::try_from(self.rng.below(501)).unwrap_or(0)).ok()
                })
                .flatten(),
            provenance,
        }
    }

    fn header(&mut self, circle: &CircleInfo, label: Confidentiality) -> ObjectHeader {
        ObjectHeader {
            id: self.id::<ObjectId>(),
            circle: circle.id,
            confidentiality: label,
            server_visibility: self.visibility(label),
            provenance: self.provenance(circle),
        }
    }

    fn timestamp(&mut self, from_secs: i64, span_days: u32) -> Timestamp {
        Timestamp(self.instant(from_secs, span_days))
    }
}

/// `Synthetic <noun> #<n>`, plus a canary when the label is `restricted` or `secret`.
fn title(noun: &str, circle: usize, n: usize, label: Confidentiality) -> String {
    if label >= Confidentiality::Restricted {
        format!("Synthetic {noun} #{n} CANARY-{circle}-{n}")
    } else {
        format!("Synthetic {noun} #{n}")
    }
}

/// Generates the dataset: validated envelopes, in dependency order.
#[must_use]
pub fn generate(seed: u64, p: &Profile) -> Vec<Envelope> {
    let mut g = Gen {
        rng: Rng(seed),
        clock: 0,
        out: Vec::new(),
    };
    let principals: Vec<PrincipalId> = (0..p.principals).map(|_| g.id()).collect();

    // Circles: the first four form a chain so nesting depth is at least 3.
    let mut circles: Vec<CircleInfo> = Vec::new();
    for index in 0..p.circles {
        let id: CircleId = g.id();
        let parent = match index {
            0 => None,
            1..=3 => Some(circles[index - 1].id),
            _ => Some(circles[g.rng.idx(index)].id),
        };
        g.push(Object::Circle(Circle {
            id,
            name: format!("Synthetic Circle {index:02}"),
            parent,
        }));
        circles.push(CircleInfo {
            id,
            index,
            sources: vec![],
            persons: vec![],
        });
    }

    memberships(&mut g, &circles, &principals);
    sources(&mut g, &mut circles, &principals, p.sources_per_circle);
    persons(
        &mut g,
        &mut circles,
        &principals,
        p.principals + p.extra_persons,
    );

    for circle in &circles {
        for n in 0..p.objects_per_circle {
            let label = g.label();
            let header = g.header(circle, label);
            let t = title("item", circle.index, n, label);
            let object = match n % 3 {
                0 => task(&mut g, circle, header, t),
                1 => event(&mut g, circle, header, t),
                _ => document(&mut g, circle, header, t),
            };
            g.push(object);
        }
    }
    g.out
}

/// One owner plus up to 3 other distinct principals per circle.
fn memberships(g: &mut Gen, circles: &[CircleInfo], principals: &[PrincipalId]) {
    for circle in circles {
        let mut members = vec![(principals[g.rng.idx(principals.len())], Role::Owner)];
        for _ in 0..=g.rng.below(3) {
            let candidate = principals[g.rng.idx(principals.len())];
            if members.iter().all(|(m, _)| *m != candidate) {
                let role = if g.rng.chance(70) {
                    Role::Member
                } else {
                    Role::Guest
                };
                members.push((candidate, role));
            }
        }
        for (principal, role) in members {
            let since = g.timestamp(BASE_SECS - 90 * DAY, 60);
            g.push(Object::Membership(Membership {
                circle: circle.id,
                principal,
                role,
                since,
            }));
        }
    }
}

fn sources(g: &mut Gen, circles: &mut [CircleInfo], principals: &[PrincipalId], per_circle: usize) {
    let kinds = [
        SourceKind::Text,
        SourceKind::Pdf,
        SourceKind::Image,
        SourceKind::Email,
        SourceKind::Voice,
    ];
    for circle in circles {
        for n in 0..per_circle {
            let id: SourceId = g.id();
            let captured_at = g.instant(BASE_SECS - 30 * DAY, 30);
            let confidentiality = g.label();
            let source = Source {
                id,
                circle: circle.id,
                kind: kinds[(circle.index + n) % kinds.len()],
                captured_at: Timestamp(captured_at),
                captured_by: principals[g.rng.idx(principals.len())],
                confidentiality,
                server_visibility: g.visibility(confidentiality),
            };
            g.push(Object::Source(source));
            circle.sources.push((id, captured_at));
        }
    }
}

/// Principal-backed persons first, then people without an account (e.g. children).
/// A person's identity is its object id; `PersonId` is the same UUID (schema v0).
fn persons(g: &mut Gen, circles: &mut [CircleInfo], principals: &[PrincipalId], count: usize) {
    let n_circles = circles.len();
    for k in 0..count {
        let circle = &mut circles[k % n_circles];
        let label = g.label();
        let header = g.header(circle, label);
        let person_id =
            PersonId::try_from(header.id.as_uuid()).unwrap_or_else(|_| unreachable!("v7"));
        let display_name = g.fact(format!("Persona-{k:03}"), circle);
        g.push(Object::Person(Person {
            header,
            display_name,
            principal: principals.get(k).copied(),
        }));
        circle.persons.push(person_id);
    }
}

fn people(g: &mut Gen, circle: &CircleInfo) -> Vec<Fact<PersonId>> {
    if circle.persons.is_empty() {
        return vec![];
    }
    (0..g.rng.below(3))
        .map(|_| {
            let person = circle.persons[g.rng.idx(circle.persons.len())];
            g.fact(person, circle)
        })
        .collect()
}

fn task(g: &mut Gen, circle: &CircleInfo, header: ObjectHeader, title: String) -> Object {
    let due = if g.rng.chance(60) {
        let at = g.timestamp(BASE_SECS, 90);
        Some(g.fact(at, circle))
    } else {
        None
    };
    let status = match g.rng.below(3) {
        0 => TaskStatus::Open,
        1 => TaskStatus::Done,
        _ => TaskStatus::Cancelled,
    };
    Object::Task(Task {
        title: g.fact(title, circle),
        due,
        assignees: people(g, circle),
        status,
        header,
    })
}

fn event(g: &mut Gen, circle: &CircleInfo, header: ObjectHeader, title: String) -> Object {
    let start = g.instant(BASE_SECS, 90);
    let ends_at = if g.rng.chance(60) {
        let end = start + time::Duration::minutes(i64::from(30 + g.rng.below(150)));
        Some(g.fact(Timestamp(end), circle))
    } else {
        None
    };
    Object::Event(Event {
        title: g.fact(title, circle),
        starts_at: g.fact(Timestamp(start), circle),
        ends_at,
        participants: people(g, circle),
        header,
    })
}

fn document(g: &mut Gen, circle: &CircleInfo, header: ObjectHeader, title: String) -> Object {
    let roles = [
        DateRole::Issued,
        DateRole::Due,
        DateRole::Appointment,
        DateRole::Payment,
    ];
    let dates = (0..=g.rng.below(2))
        .map(|_| {
            let role = roles[g.rng.idx(roles.len())];
            let at = g.timestamp(BASE_SECS - 30 * DAY, 120);
            g.fact(DatedValue { role, at }, circle)
        })
        .collect();
    let source = if circle.sources.is_empty() {
        g.id()
    } else {
        circle.sources[g.rng.idx(circle.sources.len())].0
    };
    Object::Document(Document {
        title: g.fact(title, circle),
        source,
        dates,
        header,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use serde_json::Value;

    use super::*;

    fn lines(seed: u64, p: &Profile) -> Vec<String> {
        generate(seed, p)
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect()
    }

    fn values(seed: u64, p: &Profile) -> Vec<Value> {
        lines(seed, p)
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn of_type<'a>(vs: &'a [Value], t: &'a str) -> impl Iterator<Item = &'a Value> {
        vs.iter()
            .map(|v| &v["object"])
            .filter(move |o| o["type"] == t)
    }

    fn collect_sources(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                for (k, v) in m {
                    if k == "source" {
                        out.push(v.as_str().unwrap().to_owned());
                    } else {
                        collect_sources(v, out);
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|v| collect_sources(v, out)),
            _ => {}
        }
    }

    #[test]
    fn same_seed_same_bytes_other_seed_differs() {
        assert_eq!(lines(42, &SMALL), lines(42, &SMALL));
        assert_ne!(lines(42, &SMALL), lines(43, &SMALL));
    }

    #[test]
    fn every_line_reparses_as_an_envelope() {
        for p in [&SMALL, &LARGE] {
            for line in lines(7, p) {
                let env: Envelope = serde_json::from_str(&line).unwrap();
                assert_eq!(serde_json::to_string(&env).unwrap(), line, "{}", p.name);
            }
        }
    }

    #[test]
    fn cardinalities_match_the_declared_profile() {
        for p in [&SMALL, &LARGE] {
            let vs = values(1, p);
            assert_eq!(of_type(&vs, "circle").count(), p.circles);
            assert_eq!(
                of_type(&vs, "source").count(),
                p.circles * p.sources_per_circle
            );
            assert_eq!(
                of_type(&vs, "person").count(),
                p.principals + p.extra_persons
            );
            let objects = ["task", "event", "document"]
                .iter()
                .map(|t| of_type(&vs, t).count())
                .sum::<usize>();
            assert_eq!(objects, p.circles * p.objects_per_circle);
        }
    }

    #[test]
    fn ids_are_unique() {
        let vs = values(3, &LARGE);
        let mut seen = HashSet::new();
        for o in vs.iter().map(|v| &v["object"]) {
            let id = if o["type"] == "circle" || o["type"] == "source" {
                &o["id"]
            } else {
                &o["header"]["id"]
            };
            if !id.is_null() {
                assert!(seen.insert(id.as_str().unwrap().to_owned()));
            }
        }
    }

    #[test]
    fn references_resolve_and_never_cross_circles() {
        for seed in [1, 2, 3] {
            let vs = values(seed, &SMALL);
            let circles: HashSet<&str> = of_type(&vs, "circle")
                .map(|c| c["id"].as_str().unwrap())
                .collect();
            for c in of_type(&vs, "circle") {
                if let Some(parent) = c["parent"].as_str() {
                    assert!(circles.contains(parent));
                }
            }
            let source_circle: HashMap<&str, &str> = of_type(&vs, "source")
                .map(|s| (s["id"].as_str().unwrap(), s["circle"].as_str().unwrap()))
                .collect();
            let person_circle: HashMap<&str, &str> = of_type(&vs, "person")
                .map(|p| {
                    (
                        p["header"]["id"].as_str().unwrap(),
                        p["header"]["circle"].as_str().unwrap(),
                    )
                })
                .collect();
            for m in of_type(&vs, "membership") {
                assert!(circles.contains(m["circle"].as_str().unwrap()));
            }
            for o in vs
                .iter()
                .map(|v| &v["object"])
                .filter(|o| !o["header"].is_null())
            {
                let circle = o["header"]["circle"].as_str().unwrap();
                assert!(circles.contains(circle));
                let mut refs = vec![];
                collect_sources(o, &mut refs);
                for s in refs {
                    assert_eq!(
                        source_circle.get(s.as_str()),
                        Some(&circle),
                        "source {s} must exist in the same circle"
                    );
                }
                for key in ["assignees", "participants"] {
                    for f in o[key].as_array().into_iter().flatten() {
                        assert_eq!(
                            person_circle.get(f["value"].as_str().unwrap()),
                            Some(&circle)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn covers_every_label_visibility_kind_and_date_role() {
        for seed in [1, 2, 3] {
            let vs = values(seed, &SMALL);
            let objs: Vec<&Value> = vs.iter().map(|v| &v["object"]).collect();
            let field = |key: &str| -> HashSet<String> {
                objs.iter()
                    .flat_map(|o| [&o[key], &o["header"][key]])
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            };
            assert_eq!(field("confidentiality").len(), 4);
            assert_eq!(field("server_visibility").len(), 3);
            assert_eq!(
                of_type(&vs, "source")
                    .map(|s| s["kind"].as_str().unwrap())
                    .collect::<HashSet<_>>()
                    .len(),
                5
            );
            let roles: HashSet<_> = of_type(&vs, "document")
                .flat_map(|d| {
                    d["dates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|f| f["value"]["role"].as_str().unwrap())
                })
                .collect();
            assert!(!roles.is_empty());
        }
        let vs = values(1, &LARGE);
        let roles: HashSet<_> = of_type(&vs, "document")
            .flat_map(|d| {
                d["dates"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|f| f["value"]["role"].as_str().unwrap())
            })
            .collect();
        assert_eq!(roles.len(), 4);
    }

    fn depth<'a>(parent: &HashMap<&'a str, Option<&'a str>>, mut id: &'a str) -> u32 {
        let mut d = 0;
        while let Some(Some(p)) = parent.get(id) {
            id = p;
            d += 1;
        }
        d
    }

    #[test]
    fn nesting_depth_is_at_least_three() {
        let vs = values(9, &SMALL);
        let parent: HashMap<&str, Option<&str>> = of_type(&vs, "circle")
            .map(|c| (c["id"].as_str().unwrap(), c["parent"].as_str()))
            .collect();
        assert!(parent.keys().map(|k| depth(&parent, k)).max().unwrap() >= 3);
    }

    #[test]
    fn canary_marks_exactly_restricted_and_secret_and_they_are_never_server_readable() {
        for seed in [1, 2] {
            let vs = values(seed, &SMALL);
            let mut canaries = 0;
            for o in vs
                .iter()
                .map(|v| &v["object"])
                .filter(|o| !o["header"].is_null())
            {
                let label = o["header"]["confidentiality"].as_str().unwrap();
                let restricted = label == "restricted" || label == "secret";
                if restricted {
                    assert_eq!(o["header"]["server_visibility"], "end_to_end_encrypted");
                }
                if o["type"] == "person" {
                    continue; // names are `Persona-NNN`, not titled
                }
                let t = o["title"]["value"].as_str().unwrap();
                assert_eq!(t.contains("CANARY-"), restricted, "{t}");
                canaries += usize::from(restricted);
            }
            assert!(canaries > 0);
        }
    }
}
