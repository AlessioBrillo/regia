//! Regia domain core.
//!
//! Only decisions the technical report marks as already taken (§14.1) live here:
//! stable typed identities, confidentiality labels, an explicit server-visibility
//! declaration per object, and first-class provenance, plus the logical entity schema v0
//! ([`Envelope`]): a wire contract, *not* a storage decision (ADR-002/003).
//! Sync, policy engine and cryptography are deliberately absent: they are open ADRs.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod entity;
pub mod envelope;
pub mod id;
pub mod label;
pub mod object;
pub mod provenance;
/// Deterministic synthetic datasets (DAT-01). Feature `synthetic`; never a production dependency.
#[cfg(feature = "synthetic")]
pub mod synthetic;
pub mod visibility;

pub use entity::{
    Circle, DateRole, DatedValue, Document, Event, Membership, Person, Role, Source, SourceKind,
    Task, TaskStatus, Timestamp,
};
pub use envelope::{Envelope, Object, SCHEMA_VERSION, SchemaError};
pub use id::{CircleId, DeviceId, IdError, ObjectId, PersonId, PrincipalId, SourceId};
pub use label::{Confidentiality, UnknownLabel};
pub use object::ObjectHeader;
pub use provenance::{
    Assertion, Confidence, ConfidenceOutOfRange, Fact, Locator, Provenance, SourceRef,
    UnsourcedReason,
};
pub use visibility::{ServerVisibility, UnknownVisibility};
