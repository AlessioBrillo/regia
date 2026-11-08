//! First-class provenance (AR-06, report §8.3: "100% of objects have a source or a declared reason").
//!
//! A fact without a source *and* without a reason cannot be represented. Missing data is
//! `Option::None`, not a fact.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::id::SourceId;

/// Where inside a source a value was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Locator {
    /// 1-based page number.
    Page(u32),
    /// Character span `[start, end)` in the extracted text.
    CharSpan {
        /// Inclusive start.
        start: u32,
        /// Exclusive end.
        end: u32,
    },
}

/// A pointer from a value back to the source it was derived from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    /// The source.
    pub source: SourceId,
    /// Position inside the source, when known.
    pub locator: Option<Locator>,
    /// When the source content was captured.
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: OffsetDateTime,
}

/// Error for a blank "no source" justification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("an unsourced fact needs a non-blank reason")]
pub struct BlankReason;

/// Why a fact legitimately has no source (e.g. "typed by the user"). Never blank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct UnsourcedReason(String);

impl UnsourcedReason {
    /// Creates a reason, trimming it; fails if nothing remains.
    ///
    /// # Errors
    /// [`BlankReason`] if the trimmed text is empty.
    pub fn new(reason: &str) -> Result<Self, BlankReason> {
        let r = reason.trim();
        if r.is_empty() {
            Err(BlankReason)
        } else {
            Ok(Self(r.to_owned()))
        }
    }

    /// The reason text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for UnsourcedReason {
    type Error = BlankReason;

    fn try_from(s: String) -> Result<Self, BlankReason> {
        Self::new(&s)
    }
}

impl From<UnsourcedReason> for String {
    fn from(r: UnsourcedReason) -> String {
        r.0
    }
}

/// Source evidence for a value: at least one source, or an explicit reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// Derived from one or more sources (non-empty by construction).
    Sourced {
        /// First (mandatory) source.
        first: SourceRef,
        /// Additional sources.
        #[serde(default)]
        more: Vec<SourceRef>,
    },
    /// No source exists, for a stated reason.
    Unsourced {
        /// Justification.
        reason: UnsourcedReason,
    },
}

impl Provenance {
    /// All source references (empty for [`Provenance::Unsourced`]).
    pub fn sources(&self) -> impl Iterator<Item = &SourceRef> {
        let (first, more) = match self {
            Self::Sourced { first, more } => (Some(first), more.as_slice()),
            Self::Unsourced { .. } => (None, [].as_slice()),
        };
        first.into_iter().chain(more)
    }
}

/// Confirmation state of a value. Absence of a value is `Option::None`, not a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Assertion {
    /// Extracted or inferred; not yet accepted by a person.
    Proposed,
    /// Accepted or entered by a person.
    Confirmed,
}

/// Error for a confidence outside `0..=1000`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("confidence must be in 0..=1000 per mille, got {0}")]
pub struct ConfidenceOutOfRange(pub u16);

/// Extractor-reported confidence in per mille (`0..=1000`).
///
/// Not a calibrated probability (report §4.8): informational only, never an input to
/// authorisation or to "send to cloud" decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Confidence(u16);

impl Confidence {
    /// Creates a confidence value.
    ///
    /// # Errors
    /// [`ConfidenceOutOfRange`] if `per_mille > 1000`.
    pub const fn new(per_mille: u16) -> Result<Self, ConfidenceOutOfRange> {
        if per_mille <= 1000 {
            Ok(Self(per_mille))
        } else {
            Err(ConfidenceOutOfRange(per_mille))
        }
    }

    /// The value in per mille.
    #[must_use]
    pub const fn per_mille(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for Confidence {
    type Error = ConfidenceOutOfRange;

    fn try_from(v: u16) -> Result<Self, ConfidenceOutOfRange> {
        Self::new(v)
    }
}

impl From<Confidence> for u16 {
    fn from(c: Confidence) -> u16 {
        c.0
    }
}

/// A value together with how sure we are and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fact<T> {
    /// The value.
    pub value: T,
    /// Confirmation state.
    pub assertion: Assertion,
    /// Evidence.
    pub provenance: Provenance,
    /// Extractor confidence, when one was reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> SourceRef {
        SourceRef {
            source: SourceId::new(),
            locator: Some(Locator::Page(2)),
            captured_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn blank_reason_is_unrepresentable() {
        assert!(UnsourcedReason::new("  \n\t").is_err());
        assert_eq!(
            UnsourcedReason::new(" typed by user ").unwrap().as_str(),
            "typed by user"
        );
        assert!(serde_json::from_str::<UnsourcedReason>("\"   \"").is_err());
        assert!(serde_json::from_str::<Provenance>(r#"{"unsourced":{"reason":""}}"#).is_err());
    }

    #[test]
    fn sourced_without_a_source_is_unrepresentable() {
        assert!(serde_json::from_str::<Provenance>(r#"{"sourced":{"more":[]}}"#).is_err());
        assert!(serde_json::from_str::<Provenance>(r#"{"sourced":{}}"#).is_err());
    }

    #[test]
    fn sources_iterates_first_then_more() {
        let p = Provenance::Sourced {
            first: src(),
            more: vec![src(), src()],
        };
        assert_eq!(p.sources().count(), 3);
        let u = Provenance::Unsourced {
            reason: UnsourcedReason::new("typed").unwrap(),
        };
        assert_eq!(u.sources().count(), 0);
    }

    #[test]
    fn fact_json_roundtrip() {
        let f = Fact {
            value: "2026-11-30".to_owned(),
            assertion: Assertion::Proposed,
            provenance: Provenance::Sourced {
                first: src(),
                more: vec![],
            },
            confidence: Some(Confidence::new(870).unwrap()),
        };
        let json = serde_json::to_string(&f).unwrap();
        assert_eq!(f, serde_json::from_str::<Fact<String>>(&json).unwrap());
    }

    #[test]
    fn confidence_is_bounded() {
        assert!(Confidence::new(1000).is_ok());
        assert_eq!(Confidence::new(1001), Err(ConfidenceOutOfRange(1001)));
        assert!(serde_json::from_str::<Confidence>("1001").is_err());
        assert!(serde_json::from_str::<Confidence>("-1").is_err());
        assert_eq!(
            serde_json::from_str::<Confidence>("0").unwrap().per_mille(),
            0
        );
    }
}
