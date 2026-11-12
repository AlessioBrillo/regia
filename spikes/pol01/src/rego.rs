//! Rego adapter via `regorus` (an OPA-compatible interpreter in Rust; not OPA itself). Every failure is a deny.

use std::cell::RefCell;

use regia_domain::Role;
use serde_json::{Value, json};

use crate::{Decision, Engine, Origin, Request};

const POLICY: &str = include_str!("../policies/regia.rego");

/// Rego-backed engine.
pub struct RegoEngine {
    // regorus evaluation needs `&mut`; the spike is single-threaded.
    engine: RefCell<regorus::Engine>,
    version: String,
}

fn role(r: Role) -> &'static str {
    match r {
        Role::Owner => "owner",
        Role::Member => "member",
        Role::Guest => "guest",
        // Role is non-exhaustive: an unknown role matches no rule.
        _ => "unknown",
    }
}

fn input(r: &Request) -> Value {
    let mut resource = json!({
        "circle": r.resource.circle.to_string(),
        "ancestors": r.resource.ancestors.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "grantees": r.resource.grantees.iter().map(ToString::to_string).collect::<Vec<_>>(),
    });
    // Absent attributes stay absent (never null) so Rego sees them as undefined.
    if let Some(l) = &r.resource.label {
        resource["label"] = json!(l);
    }
    if let Some(o) = &r.resource.owner {
        resource["owner"] = json!(o.to_string());
    }
    if let Some(c) = &r.resource.category {
        resource["category"] = json!(c);
    }
    json!({
        "principal": {
            "id": r.principal.id.to_string(),
            "memberships": r.principal.memberships.iter().map(|m| json!({
                "circle": m.circle.to_string(), "role": role(m.role), "revoked": m.revoked,
            })).collect::<Vec<_>>(),
        },
        "action": r.action.as_str(),
        "resource": resource,
        "context": {
            "untrusted": r.context.origin == Origin::Untrusted,
            "approved": r.context.approved,
        },
    })
}

impl RegoEngine {
    /// Parses the bundled policy. An invalid policy yields no engine.
    ///
    /// # Errors
    /// The policy text does not compile.
    pub fn new(version: &str) -> Result<Self, String> {
        Self::from_source(POLICY, version)
    }

    /// As [`Self::new`] with explicit source (used by the malformed-policy test).
    ///
    /// # Errors
    /// See [`Self::new`].
    pub fn from_source(policy: &str, version: &str) -> Result<Self, String> {
        let mut engine = regorus::Engine::new();
        engine
            .add_policy("regia.rego".to_owned(), policy.to_owned())
            .map_err(|e| e.to_string())?;
        Ok(Self {
            engine: RefCell::new(engine),
            version: version.to_owned(),
        })
    }

    fn evaluate(&self, r: &Request) -> Result<Decision, String> {
        let mut engine = self.engine.borrow_mut();
        engine.set_input(
            regorus::Value::from_json_str(&input(r).to_string()).map_err(|e| e.to_string())?,
        );
        let allow = engine
            .eval_rule("data.regia.allow".to_owned())
            .map_err(|e| e.to_string())?;
        let reasons = engine
            .eval_rule("data.regia.reasons".to_owned())
            .map_err(|e| e.to_string())?;
        let reasons: Vec<String> =
            serde_json::from_str(&reasons.to_json_str().map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        Ok(Decision {
            allow: allow == regorus::Value::from(true),
            reasons,
            policy_version: self.version.clone(),
        })
    }
}

impl Engine for RegoEngine {
    fn name(&self) -> &'static str {
        "rego"
    }

    fn decide(&self, request: &Request) -> Decision {
        self.evaluate(request)
            .unwrap_or_else(|e| Decision::deny(format!("deny_engine_error: {e}"), &self.version))
    }
}
