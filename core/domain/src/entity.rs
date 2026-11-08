//! Core entities, logical schema v0 (WP1).
//!
//! This is a *wire/logical* schema, not a storage decision: there is no revision, clock or
//! merge metadata here because ADR-002/003 are open. Every struct rejects unknown fields and has
//! no `Default`, so nothing can be implied.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    id::{CircleId, PrincipalId, SourceId},
    label::Confidentiality,
    object::ObjectHeader,
    provenance::Fact,
    visibility::ServerVisibility,
};

/// An instant serialised as RFC 3339 (offset kept, so the original wall clock round-trips).
///
/// `ponytail:` fixed offset, not an IANA zone; add the zone with WF-05 (reminders / DST).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] pub OffsetDateTime);

/// A circle (M7). Circles may nest via `parent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Circle {
    /// Stable identity.
    pub id: CircleId,
    /// Human-readable name (non-blank, checked by the envelope).
    pub name: String,
    /// Enclosing circle, if any.
    pub parent: Option<CircleId>,
}

/// Role inside a circle. Names are provisional until ADR-005 fixes the taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Role {
    /// Full control of the circle.
    Owner,
    /// Regular participant.
    Member,
    /// Limited, explicitly scoped access.
    Guest,
}

/// A principal's membership in a circle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Membership {
    /// The circle.
    pub circle: CircleId,
    /// The member.
    pub principal: PrincipalId,
    /// Role held.
    pub role: Role,
    /// When the membership started.
    pub since: Timestamp,
}

/// A person in the graph. May be a child or other non-user, hence `principal` is optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Person {
    /// Mandatory object metadata.
    pub header: ObjectHeader,
    /// Display name, with its evidence.
    pub display_name: Fact<String>,
    /// The acting identity of this person, if they have one.
    pub principal: Option<PrincipalId>,
}

/// What kind of input a [`Source`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceKind {
    /// Typed or pasted text.
    Text,
    /// A PDF document.
    Pdf,
    /// A photo or scan.
    Image,
    /// A forwarded email.
    Email,
    /// A voice capture.
    Voice,
}

/// Where facts come from. It is the *origin* of provenance, so it carries its own label and
/// visibility instead of an [`ObjectHeader`] (which would require provenance of provenance).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Stable identity.
    pub id: SourceId,
    /// Owning circle.
    pub circle: CircleId,
    /// Input kind.
    pub kind: SourceKind,
    /// When it was captured.
    pub captured_at: Timestamp,
    /// Who captured it.
    pub captured_by: PrincipalId,
    /// Confidentiality label.
    pub confidentiality: Confidentiality,
    /// What the server side may see.
    pub server_visibility: ServerVisibility,
}
