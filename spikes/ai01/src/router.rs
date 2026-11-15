//! Routing on metadata only (R-01..R-08). The prompt does not exist yet when this runs.
//!
//! A fallback is allowed only to another *local* runtime. Nothing here ever selects a cloud runtime that the caller
//! did not ask for, and an error is a refusal, never an `allow` (AGENTS.md).

use std::time::Instant;

use regia_domain::Confidentiality;
use serde::Serialize;

use crate::{
    contract::{Adapter, CONTRACT_VERSION, TaskRequest, TaskResult, Validation},
    validate::validate,
};

/// A runtime the router may choose from.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Adapter id.
    pub id: String,
    /// Runs on this device.
    pub local: bool,
}

/// The only facts the router may look at.
#[derive(Debug, Clone, Copy)]
pub struct Meta {
    /// Confidentiality label.
    pub label: Confidentiality,
    /// A `CANARY-` marker is present in the input.
    pub has_canary: bool,
    /// Needs to work offline.
    pub offline_required: bool,
    /// User consented to a cloud runtime.
    pub cloud_consent: bool,
}

impl Meta {
    /// Extracts the metadata. The only place the input text is inspected before routing.
    #[must_use]
    pub fn of(req: &TaskRequest) -> Self {
        Self {
            label: req.label,
            has_canary: req.input.contains("CANARY-"),
            offline_required: req.offline_required,
            cloud_consent: req.cloud_consent,
        }
    }
}

/// Why nothing ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// `restricted` and `secret` never go to a cloud runtime.
    LabelForbidsCloud,
    /// A canary marker never goes to a cloud runtime.
    CanaryForbidsCloud,
    /// Cloud needs explicit consent for this task.
    NeedsConsent,
    /// The task must work offline and the chosen runtime is cloud.
    OfflineRequiresLocal,
    /// No usable runtime on this device.
    Unavailable,
}

/// The chosen runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// The preferred runtime.
    Use(String),
    /// A local runtime instead of an unavailable preferred one.
    Fallback {
        /// Runtime used.
        to: String,
        /// Why, for the audit.
        reason: String,
    },
}

/// Decides the runtime from metadata.
///
/// # Errors
/// A [`Refusal`] when the task may not run as asked.
pub fn route(meta: &Meta, preferred: &str, available: &[Candidate]) -> Result<Route, Refusal> {
    let Some(chosen) = available.iter().find(|c| c.id == preferred) else {
        // Never fall back to cloud: that would change the data treatment (R-02, R-03).
        return available
            .iter()
            .find(|c| c.local)
            .map(|c| Route::Fallback {
                to: c.id.clone(),
                reason: format!("{preferred} unavailable"),
            })
            .ok_or(Refusal::Unavailable);
    };
    if !chosen.local {
        if meta.offline_required {
            return Err(Refusal::OfflineRequiresLocal);
        }
        if meta.label >= Confidentiality::Restricted {
            return Err(Refusal::LabelForbidsCloud);
        }
        if meta.has_canary {
            return Err(Refusal::CanaryForbidsCloud);
        }
        if !meta.cloud_consent {
            return Err(Refusal::NeedsConsent);
        }
    }
    Ok(Route::Use(chosen.id.clone()))
}

/// What is logged about a routing decision: never the input (AR-12, R-06).
#[derive(Debug, Clone, Serialize)]
pub struct AuditRecord {
    /// Runtime id, or the preferred one when refused.
    pub runtime: String,
    /// Adapter version.
    pub version: String,
    /// Contract version.
    pub contract: &'static str,
    /// Label of the input.
    pub label: Confidentiality,
    /// Decision and reason.
    pub reason: String,
}

/// Router, adapters and validation in one place.
#[derive(Default)]
pub struct Gateway {
    adapters: Vec<Box<dyn Adapter>>,
}

impl Gateway {
    /// Registers an adapter.
    pub fn add(&mut self, a: Box<dyn Adapter>) {
        self.adapters.push(a);
    }

    fn candidates(&self) -> Vec<Candidate> {
        self.adapters
            .iter()
            .map(|a| Candidate {
                id: a.id().to_owned(),
                local: a.local(),
            })
            .collect()
    }

    /// Routes, runs and validates. Always returns an audit record without any input text.
    ///
    /// The result is a [`Refusal`] when the task may not run; the audit record is returned either way.
    pub fn run(
        &self,
        req: &TaskRequest,
        preferred: &str,
    ) -> (Result<TaskResult, Refusal>, AuditRecord) {
        let meta = Meta::of(req);
        let audit = |runtime: &str, version: &str, reason: String| AuditRecord {
            runtime: runtime.to_owned(),
            version: version.to_owned(),
            contract: CONTRACT_VERSION,
            label: req.label,
            reason,
        };
        let route = match route(&meta, preferred, &self.candidates()) {
            Ok(r) => r,
            Err(e) => return (Err(e), audit(preferred, "-", format!("refused: {e:?}"))),
        };
        let (id, fallback_reason) = match route {
            Route::Use(id) => (id, None),
            Route::Fallback { to, reason } => (to, Some(reason)),
        };
        let Some(a) = self.adapters.iter().find(|a| a.id() == id) else {
            return (
                Err(Refusal::Unavailable),
                audit(&id, "-", "refused: missing".into()),
            );
        };
        let started = Instant::now();
        let out = a.extract(req);
        let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let (proposal, validation) = match out {
            Ok(p) => match validate(req, &p) {
                Ok(()) => (Some(p), Validation::Valid),
                Err(e) => (None, Validation::Failed(e)),
            },
            Err(e) => (None, Validation::Failed(format!("runtime error: {e}"))),
        };
        let reason = fallback_reason.clone().unwrap_or_else(|| "routed".into());
        let result = TaskResult {
            proposal,
            runtime: id,
            validation,
            latency_ms,
            fallback_reason,
        };
        (Ok(result), audit(a.id(), a.version(), reason))
    }
}
