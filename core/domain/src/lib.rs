//! Regia domain core.
//!
//! Only decisions the technical report marks as already taken (§14.1) live here:
//! stable typed identities, confidentiality labels, an explicit server-visibility
//! declaration per object, and first-class provenance. Sync, policy engine and
//! cryptography are deliberately absent: they are open ADRs.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod id;
pub mod label;
pub mod object;
pub mod provenance;
pub mod visibility;

pub use id::{CircleId, DeviceId, IdError, ObjectId, PersonId, PrincipalId, SourceId};
pub use label::{Confidentiality, UnknownLabel};
pub use object::ObjectHeader;
pub use provenance::{
    Assertion, Confidence, ConfidenceOutOfRange, Fact, Locator, Provenance, SourceRef,
    UnsourcedReason,
};
pub use visibility::{ServerVisibility, UnknownVisibility};
