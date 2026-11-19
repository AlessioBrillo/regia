//! AI-01 spike: a versioned task contract, a router that decides on metadata only, and replaceable adapters.
//!
//! Throwaway code: nothing here is a decision (ADR-008/009 are pending). The model never authorises anything;
//! deterministic code validates every proposal against the input before it can reach a user (report §4.8, AR-06).

pub mod contract;
pub mod llm;
pub mod r0;
pub mod r3;
pub mod r4;
pub mod router;
pub mod score;
pub mod validate;

pub use contract::{
    Adapter, Dated, Expect, Golden, Obj, ObjKind, Outcome, Proposal, TaskRequest, TaskResult,
    Validation,
};
pub use router::{AuditRecord, Candidate, Gateway, Meta, Refusal, Route, route};
