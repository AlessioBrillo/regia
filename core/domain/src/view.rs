//! Read-side enforcement of confidentiality rules that are already decided (§4.5, AR-04).
//!
//! A view is deliberately small: it knows which circles the reader holds and applies the
//! decided label rule (`restricted` and `secret` never leave the device). It is **not** a
//! policy engine (ADR-005), does not make authorisation decisions from model output and does
//! not decide anything about search or indexes (ADR-011). An error is always a withholding,
//! never an implicit allow.

use std::collections::HashSet;

use crate::{CircleId, Confidentiality, Envelope, Object};

/// Why an object was withheld. The reason is metadata, never the object's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Withheld {
    /// `restricted` or `secret`: readable only on the device, so never rendered here.
    Label,
    /// The object belongs to a circle the reader does not hold.
    Circle,
}

/// The outcome of evaluating one envelope against a [`ReadView`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadDecision {
    /// The object may be rendered with its provenance.
    Show,
    /// The object is withheld; only [`Withheld`] is recorded.
    Withheld(Withheld),
}

/// The circles a reader holds. No keys, no credentials and no personal data live here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadView {
    allowed: HashSet<CircleId>,
}

impl ReadView {
    /// A view holding exactly one circle, as used by the local owner in the demo slice.
    #[must_use]
    pub fn owner(circle: CircleId) -> Self {
        Self {
            allowed: HashSet::from([circle]),
        }
    }

    /// A view holding the given circles.
    #[must_use]
    pub fn with_circles<I>(circles: I) -> Self
    where
        I: IntoIterator<Item = CircleId>,
    {
        Self {
            allowed: circles.into_iter().collect(),
        }
    }

    /// The circles held by this view.
    #[must_use]
    pub const fn circles(&self) -> &HashSet<CircleId> {
        &self.allowed
    }

    /// Decides whether `envelope` may be shown, checking the label first so that a
    /// `restricted`/`secret` object is withheld even when its circle is held.
    #[must_use]
    pub fn evaluate(&self, envelope: &Envelope) -> ReadDecision {
        let (circle, label) = match envelope.object() {
            Object::Person(p) => (Some(p.header.circle), Some(p.header.confidentiality)),
            Object::Task(t) => (Some(t.header.circle), Some(t.header.confidentiality)),
            Object::Event(e) => (Some(e.header.circle), Some(e.header.confidentiality)),
            Object::Document(d) => (Some(d.header.circle), Some(d.header.confidentiality)),
            Object::Membership(m) => (Some(m.circle), None),
            Object::Source(s) => (Some(s.circle), Some(s.confidentiality)),
            Object::Circle(c) => (Some(c.id), None),
            Object::Area(_) => (None, None),
        };
        if label.is_some_and(|label| label >= Confidentiality::Restricted) {
            return ReadDecision::Withheld(Withheld::Label);
        }
        if circle.is_some_and(|circle| !self.allowed.contains(&circle)) {
            return ReadDecision::Withheld(Withheld::Circle);
        }
        ReadDecision::Show
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;
    use crate::{
        Assertion, CircleId, Confidentiality, Fact, ObjectHeader, ObjectId, Provenance,
        ServerVisibility, SourceId, SourceRef, Task, TaskStatus,
    };

    fn task(label: Confidentiality, circle: CircleId, text: &str) -> Envelope {
        let source = SourceRef {
            source: SourceId::new(),
            locator: None,
            captured_at: OffsetDateTime::UNIX_EPOCH,
        };
        let provenance = || Provenance::Sourced {
            first: source.clone(),
            more: vec![],
        };
        Envelope::new(Object::Task(Task {
            header: ObjectHeader {
                id: ObjectId::new(),
                circle,
                confidentiality: label,
                server_visibility: ServerVisibility::EndToEndEncrypted,
                area: None,
                provenance: provenance(),
            },
            title: Fact {
                value: text.to_owned(),
                assertion: Assertion::Proposed,
                provenance: provenance(),
                confidence: None,
            },
            due: None,
            assignees: vec![],
            status: TaskStatus::Open,
        }))
        .unwrap()
    }

    #[test]
    fn restricted_and_secret_with_canary_are_withheld_even_in_an_allowed_circle() {
        let circle = CircleId::new();
        let view = ReadView::owner(circle);
        for label in [Confidentiality::Restricted, Confidentiality::Secret] {
            let envelope = task(label, circle, "CANARY-1-2 secret note");
            assert_eq!(
                view.evaluate(&envelope),
                ReadDecision::Withheld(Withheld::Label)
            );
        }
    }

    #[test]
    fn other_circles_are_withheld_but_public_and_circle_are_shown() {
        let allowed = CircleId::new();
        let view = ReadView::owner(allowed);
        assert_eq!(
            view.evaluate(&task(Confidentiality::Circle, CircleId::new(), "other")),
            ReadDecision::Withheld(Withheld::Circle)
        );
        for label in [Confidentiality::Public, Confidentiality::Circle] {
            assert_eq!(
                view.evaluate(&task(label, allowed, "Synthetic: call Anna")),
                ReadDecision::Show
            );
        }
    }

    #[test]
    fn visible_output_never_contains_a_canary_from_a_withheld_object() {
        let allowed = CircleId::new();
        let view = ReadView::owner(allowed);
        let envelopes = [
            task(Confidentiality::Circle, allowed, "Synthetic: call Anna"),
            task(Confidentiality::Secret, allowed, "CANARY-9-9 hidden"),
            task(
                Confidentiality::Restricted,
                CircleId::new(),
                "CANARY-8-8 hidden",
            ),
        ];
        // This mirrors the query example: only visible envelopes are rendered.
        let rendered: String = envelopes
            .iter()
            .filter(|envelope| view.evaluate(envelope) == ReadDecision::Show)
            .filter_map(|envelope| match envelope.object() {
                Object::Task(task) => Some(task.title.value.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!rendered.contains("CANARY-"));
        assert_eq!(rendered.lines().count(), 1);
    }
}
