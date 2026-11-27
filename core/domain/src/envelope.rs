//! Versioned wire envelope: the single entry point for untrusted object payloads.
//!
//! Deserialising an [`Envelope`] validates everything the types alone cannot express. Unknown
//! schema versions are rejected, never read "best effort" (deny-by-default). A payload of a
//! future version may fail earlier as a parse error; either way it is refused.

use serde::{Deserialize, Serialize};

use crate::{
    entity::{Area, Circle, Document, Event, Membership, Person, Source, Task},
    object::ObjectHeader,
    provenance::{Assertion, Provenance},
};

/// The only schema version this crate reads and writes.
pub const SCHEMA_VERSION: u32 = 0;

/// Why a payload was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SchemaError {
    /// The payload declares a version this crate does not read.
    #[error("unsupported schema version {0} (supported: {SCHEMA_VERSION})")]
    UnsupportedVersion(u32),
    /// A mandatory text field is empty or whitespace.
    #[error("{0} must not be blank")]
    Blank(&'static str),
    /// A circle names itself as its own parent.
    #[error("a circle cannot be its own parent")]
    SelfParent,
    /// An event ends before it starts.
    #[error("event ends before it starts")]
    EndsBeforeStart,
    /// An area names itself as its own parent.
    #[error("an area cannot be its own parent")]
    AreaSelfParent,
    /// A model-proposed area has no source (AR-06).
    #[error("a proposed area needs a source")]
    ProposedAreaWithoutSource,
}

/// Any entity that can travel in an [`Envelope`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Object {
    /// See [`Circle`].
    Circle(Circle),
    /// See [`Area`].
    Area(Area),
    /// See [`Membership`].
    Membership(Membership),
    /// See [`Person`].
    Person(Person),
    /// See [`Source`].
    Source(Source),
    /// See [`Task`].
    Task(Task),
    /// See [`Event`].
    Event(Event),
    /// See [`Document`].
    Document(Document),
}

impl Object {
    fn validate(&self) -> Result<(), SchemaError> {
        match self {
            Self::Circle(c) => {
                non_blank(&c.name, "circle name")?;
                if c.parent == Some(c.id) {
                    return Err(SchemaError::SelfParent);
                }
            }
            Self::Area(a) => {
                non_blank(&a.name, "area name")?;
                if a.parent == Some(a.id) {
                    return Err(SchemaError::AreaSelfParent);
                }
            }
            Self::Person(p) => {
                validate_header(&p.header)?;
                non_blank(&p.display_name.value, "person display name")?;
            }
            Self::Task(t) => {
                validate_header(&t.header)?;
                non_blank(&t.title.value, "task title")?;
            }
            Self::Document(d) => {
                validate_header(&d.header)?;
                non_blank(&d.title.value, "document title")?;
            }
            Self::Event(e) => {
                validate_header(&e.header)?;
                non_blank(&e.title.value, "event title")?;
                if e.ends_at
                    .as_ref()
                    .is_some_and(|end| end.value < e.starts_at.value)
                {
                    return Err(SchemaError::EndsBeforeStart);
                }
            }
            Self::Membership(_) | Self::Source(_) => {}
        }
        Ok(())
    }
}

/// An area filed by the model must cite a source; a person's own choice needs only a reason.
fn validate_header(header: &ObjectHeader) -> Result<(), SchemaError> {
    if let Some(area) = &header.area
        && area.assertion == Assertion::Proposed
        && matches!(&area.provenance, Provenance::Unsourced { .. })
    {
        return Err(SchemaError::ProposedAreaWithoutSource);
    }
    Ok(())
}

fn non_blank(s: &str, what: &'static str) -> Result<(), SchemaError> {
    if s.trim().is_empty() {
        Err(SchemaError::Blank(what))
    } else {
        Ok(())
    }
}

/// A versioned, validated object. The only way to obtain one is [`Envelope::new`] or
/// deserialisation, both of which run the same checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawEnvelope")]
pub struct Envelope {
    schema_version: u32,
    object: Object,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnvelope {
    schema_version: u32,
    object: Object,
}

impl TryFrom<RawEnvelope> for Envelope {
    type Error = SchemaError;

    fn try_from(raw: RawEnvelope) -> Result<Self, SchemaError> {
        if raw.schema_version != SCHEMA_VERSION {
            return Err(SchemaError::UnsupportedVersion(raw.schema_version));
        }
        raw.object.validate()?;
        Ok(Self {
            schema_version: raw.schema_version,
            object: raw.object,
        })
    }
}

impl Envelope {
    /// Wraps an object at the current schema version after validating it.
    ///
    /// # Errors
    /// A [`SchemaError`] describing the first violated invariant.
    pub fn new(object: Object) -> Result<Self, SchemaError> {
        object.validate()?;
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            object,
        })
    }

    /// The schema version of this envelope.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// The wrapped object.
    #[must_use]
    pub const fn object(&self) -> &Object {
        &self.object
    }

    /// Consumes the envelope, returning the object.
    #[must_use]
    pub fn into_object(self) -> Object {
        self.object
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;
    use crate::{
        entity::Timestamp,
        id::{AreaId, CircleId, ObjectId, SourceId},
        label::Confidentiality,
        object::ObjectHeader,
        provenance::{Assertion, Fact, Provenance, SourceRef, UnsourcedReason},
        visibility::ServerVisibility,
    };

    fn fact<T>(value: T) -> Fact<T> {
        Fact {
            value,
            assertion: Assertion::Confirmed,
            provenance: Provenance::Unsourced {
                reason: UnsourcedReason::new("typed by user").unwrap(),
            },
            confidence: None,
        }
    }

    fn at(secs: i64) -> Timestamp {
        Timestamp(OffsetDateTime::from_unix_timestamp(secs).unwrap())
    }

    fn event(start: i64, end: Option<i64>) -> Object {
        Object::Event(Event {
            header: ObjectHeader {
                id: ObjectId::new(),
                circle: CircleId::new(),
                confidentiality: Confidentiality::Circle,
                server_visibility: ServerVisibility::EndToEndEncrypted,
                area: None,
                provenance: Provenance::Unsourced {
                    reason: UnsourcedReason::new("typed by user").unwrap(),
                },
            },
            title: fact("Synthetic school meeting".to_owned()),
            starts_at: fact(at(start)),
            ends_at: end.map(|e| fact(at(e))),
            participants: vec![],
        })
    }

    #[test]
    fn event_must_not_end_before_it_starts() {
        assert!(Envelope::new(event(100, Some(100))).is_ok());
        assert!(Envelope::new(event(100, None)).is_ok());
        assert_eq!(
            Envelope::new(event(100, Some(99))),
            Err(SchemaError::EndsBeforeStart)
        );
    }

    #[test]
    fn circle_invariants() {
        let id = CircleId::new();
        let circle = |name: &str, parent| {
            Object::Circle(Circle {
                id,
                name: name.to_owned(),
                parent,
            })
        };
        assert!(Envelope::new(circle("Synthetic Family", None)).is_ok());
        assert_eq!(
            Envelope::new(circle(" ", None)),
            Err(SchemaError::Blank("circle name"))
        );
        assert_eq!(
            Envelope::new(circle("x", Some(id))),
            Err(SchemaError::SelfParent)
        );
    }

    #[test]
    fn area_invariants() {
        let id = AreaId::new();
        let area = |name: &str, parent| {
            Object::Area(Area {
                id,
                name: name.to_owned(),
                parent,
            })
        };
        assert!(Envelope::new(area("Synthetic Work", None)).is_ok());
        assert_eq!(
            Envelope::new(area(" ", None)),
            Err(SchemaError::Blank("area name"))
        );
        assert_eq!(
            Envelope::new(area("x", Some(id))),
            Err(SchemaError::AreaSelfParent)
        );
    }

    fn area_fact(assertion: Assertion, provenance: Provenance) -> Fact<AreaId> {
        Fact {
            value: AreaId::new(),
            assertion,
            provenance,
            confidence: None,
        }
    }

    fn with_area(area: Fact<AreaId>) -> Object {
        let mut object = event(100, None);
        match &mut object {
            Object::Event(e) => e.header.area = Some(area),
            _ => unreachable!(),
        }
        object
    }

    #[test]
    fn proposed_area_needs_a_source_but_a_confirmed_choice_does_not() {
        let unsourced = || Provenance::Unsourced {
            reason: UnsourcedReason::new("model guessed").unwrap(),
        };
        assert_eq!(
            Envelope::new(with_area(area_fact(Assertion::Proposed, unsourced()))),
            Err(SchemaError::ProposedAreaWithoutSource)
        );
        assert!(Envelope::new(with_area(area_fact(Assertion::Confirmed, unsourced()))).is_ok());

        let sourced = Provenance::Sourced {
            first: SourceRef {
                source: SourceId::new(),
                locator: None,
                captured_at: OffsetDateTime::UNIX_EPOCH,
            },
            more: vec![],
        };
        assert!(Envelope::new(with_area(area_fact(Assertion::Proposed, sourced))).is_ok());
    }

    #[test]
    fn json_roundtrip_and_version_gate() {
        let env = Envelope::new(event(100, Some(200))).unwrap();
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["schema_version"], 0);
        assert_eq!(json["object"]["type"], "event");
        assert_eq!(env, serde_json::from_value(json.clone()).unwrap());

        let mut future = json;
        future["schema_version"] = serde_json::json!(1);
        let err = serde_json::from_value::<Envelope>(future).unwrap_err();
        assert!(err.to_string().contains("unsupported schema version 1"));
    }
}
