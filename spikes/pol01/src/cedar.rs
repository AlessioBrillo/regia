//! Cedar adapter. Builds a minimal entity graph per request; every failure is a deny.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    str::FromStr,
};

use cedar_policy::{
    Authorizer, Context, Decision as CedarDecision, Entities, Entity, EntityUid, PolicySet,
    Request as CedarRequest, RestrictedExpression, Schema, ValidationMode, Validator,
};

use crate::{Decision, Engine, Origin, Request};

const POLICIES: &str = include_str!("../policies/regia.cedar");
const SCHEMA: &str = include_str!("../policies/regia.cedarschema");

/// Cedar-backed engine.
pub struct CedarEngine {
    schema: Schema,
    policies: PolicySet,
    version: String,
}

type Groups = BTreeMap<(&'static str, String), HashSet<EntityUid>>;

fn group(groups: &mut Groups, ty: &'static str, id: String) -> Result<EntityUid, String> {
    groups.entry((ty, id.clone())).or_default();
    uid(ty, id)
}

fn uid(ty: &str, id: impl std::fmt::Display) -> Result<EntityUid, String> {
    EntityUid::from_str(&format!("{ty}::\"{id}\"")).map_err(|e| e.to_string())
}

impl CedarEngine {
    /// Parses and **validates** the policies against the schema. An invalid policy set yields no engine, so the
    /// caller cannot evaluate (and therefore cannot allow) anything with it.
    ///
    /// # Errors
    /// Schema or policy text does not parse, or the policies fail strict validation.
    pub fn new(version: &str) -> Result<Self, String> {
        Self::from_sources(SCHEMA, POLICIES, version)
    }

    /// As [`Self::new`] with explicit sources (used by the malformed-policy test).
    ///
    /// # Errors
    /// See [`Self::new`].
    pub fn from_sources(schema: &str, policies: &str, version: &str) -> Result<Self, String> {
        let (schema, _warnings) =
            Schema::from_cedarschema_str(schema).map_err(|e| e.to_string())?;
        let policies = PolicySet::from_str(policies).map_err(|e| e.to_string())?;
        let result = Validator::new(schema.clone()).validate(&policies, ValidationMode::Strict);
        if !result.validation_passed() {
            return Err(result
                .validation_errors()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "));
        }
        Ok(Self {
            schema,
            policies,
            version: version.to_owned(),
        })
    }

    #[allow(clippy::too_many_lines)] // throwaway spike: one linear build-then-authorise function
    fn evaluate(&self, r: &Request) -> Result<Decision, String> {
        let mut parents = Groups::new();

        // Principal: one group per active role; a revoked membership only places it in Revoked::<circle>.
        let mut principal_parents = HashSet::new();
        for m in &r.principal.memberships {
            let c = m.circle.to_string();
            let g = if m.revoked {
                group(&mut parents, "Revoked", c)?
            } else {
                match m.role {
                    regia_domain::Role::Owner => group(&mut parents, "Owners", c)?,
                    regia_domain::Role::Member => group(&mut parents, "Members", c)?,
                    regia_domain::Role::Guest => group(&mut parents, "Guests", c)?,
                    // Role is non-exhaustive: an unknown role grants nothing.
                    _ => continue,
                }
            };
            principal_parents.insert(g);
        }

        // Resource groups; ancestors' Owners hang under one AncOwners group.
        let own = r.resource.circle.to_string();
        let anc_owners = group(&mut parents, "AncOwners", "res".to_owned())?;
        for a in &r.resource.ancestors {
            let g = group(&mut parents, "Owners", a.to_string())?;
            parents
                .get_mut(&("Owners", a.to_string()))
                .ok_or("missing group")?
                .insert(anc_owners.clone());
            drop(g);
        }
        let mut attrs = HashMap::from([
            (
                "owners".to_owned(),
                RestrictedExpression::new_entity_uid(group(&mut parents, "Owners", own.clone())?),
            ),
            (
                "members".to_owned(),
                RestrictedExpression::new_entity_uid(group(&mut parents, "Members", own.clone())?),
            ),
            (
                "guests".to_owned(),
                RestrictedExpression::new_entity_uid(group(&mut parents, "Guests", own.clone())?),
            ),
            (
                "revoked".to_owned(),
                RestrictedExpression::new_entity_uid(group(&mut parents, "Revoked", own)?),
            ),
            (
                "anc_owners".to_owned(),
                RestrictedExpression::new_entity_uid(anc_owners),
            ),
            (
                "grantees".to_owned(),
                RestrictedExpression::new_set(
                    r.resource
                        .grantees
                        .iter()
                        .map(|g| uid("User", g).map(RestrictedExpression::new_entity_uid))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
        ]);
        if let Some(l) = &r.resource.label {
            attrs.insert(
                "label".to_owned(),
                RestrictedExpression::new_string(l.clone()),
            );
        }
        if let Some(o) = &r.resource.owner {
            attrs.insert(
                "owner".to_owned(),
                RestrictedExpression::new_entity_uid(uid("User", o)?),
            );
        }
        if let Some(c) = &r.resource.category {
            attrs.insert(
                "category".to_owned(),
                RestrictedExpression::new_string(c.clone()),
            );
        }

        let principal = uid("User", r.principal.id)?;
        let resource = uid("Obj", "res")?;
        let mut entities = vec![
            Entity::new(principal.clone(), HashMap::new(), principal_parents)
                .map_err(|e| e.to_string())?,
            Entity::new(resource.clone(), attrs, HashSet::new()).map_err(|e| e.to_string())?,
        ];
        // Grantees and the owner must exist as entities too.
        let others: BTreeSet<_> = r
            .resource
            .grantees
            .iter()
            .chain(&r.resource.owner)
            .filter(|u| **u != r.principal.id)
            .collect();
        for u in others {
            entities.push(Entity::new_no_attrs(uid("User", u)?, HashSet::new()));
        }
        for ((ty, id), ps) in parents {
            entities
                .push(Entity::new(uid(ty, id)?, HashMap::new(), ps).map_err(|e| e.to_string())?);
        }
        let entities =
            Entities::from_entities(entities, Some(&self.schema)).map_err(|e| e.to_string())?;

        let context = Context::from_pairs([
            (
                "untrusted".to_owned(),
                RestrictedExpression::new_bool(r.context.origin == Origin::Untrusted),
            ),
            (
                "approved".to_owned(),
                RestrictedExpression::new_bool(r.context.approved),
            ),
        ])
        .map_err(|e| e.to_string())?;
        let action = uid("Action", r.action.as_str())?;
        let request = CedarRequest::new(principal, action, resource, context, Some(&self.schema))
            .map_err(|e| e.to_string())?;

        let response = Authorizer::new().is_authorized(&request, &self.policies, &entities);
        // Cedar skips a policy that errors; a skipped forbid would fail open. Any error is a deny.
        if response.diagnostics().errors().next().is_some() {
            return Ok(Decision::deny("deny_policy_error", &self.version));
        }
        let mut reasons: Vec<String> = response
            .diagnostics()
            .reason()
            .map(ToString::to_string)
            .collect();
        reasons.sort();
        let allow = matches!(response.decision(), CedarDecision::Allow);
        if reasons.is_empty() {
            reasons.push("deny_default".to_owned());
        }
        Ok(Decision {
            allow,
            reasons,
            policy_version: self.version.clone(),
        })
    }
}

impl Engine for CedarEngine {
    fn name(&self) -> &'static str {
        "cedar"
    }

    fn decide(&self, request: &Request) -> Decision {
        self.evaluate(request)
            .unwrap_or_else(|e| Decision::deny(format!("deny_engine_error: {e}"), &self.version))
    }
}
