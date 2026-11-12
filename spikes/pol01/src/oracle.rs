//! Hand-written reference evaluator (alternative C, and the oracle for P-10).
//!
//! Rules, in order; the first matching deny wins, then an explicit allow is required:
//!
//! 1. missing/unknown label → deny; revoked membership in the resource circle → deny
//! 2. `create` of an excluded category → deny
//! 3. mutating action from untrusted origin without approval → deny
//! 4. per label:
//!    - `secret`: owner only, any action
//!    - `restricted`: owner any action; named grantees read/discover only
//!    - `circle`: Owner any action; Member read/discover/create/update/execute; Guest read/discover only if
//!      named; an Owner of an *ancestor* circle read/discover only (owner decision for P-03)
//!    - `public`: anyone read/discover; otherwise as `circle`

use regia_domain::{Confidentiality, Role};

use crate::{Action, Decision, EXCLUDED_CATEGORIES, Engine, Request};

/// The reference evaluator.
pub struct Oracle {
    version: String,
}

impl Oracle {
    /// New evaluator reporting `version`.
    #[must_use]
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_owned(),
        }
    }

    fn allow(&self, rule: &str) -> Decision {
        Decision {
            allow: true,
            reasons: vec![rule.to_owned()],
            policy_version: self.version.clone(),
        }
    }

    fn deny(&self, rule: &str) -> Decision {
        Decision::deny(rule, &self.version)
    }
}

const fn is_view(a: Action) -> bool {
    matches!(a, Action::Read | Action::Discover)
}

impl Engine for Oracle {
    fn name(&self) -> &'static str {
        "oracle"
    }

    fn decide(&self, r: &Request) -> Decision {
        let Some(label) = r
            .resource
            .label
            .as_deref()
            .and_then(|l| l.parse::<Confidentiality>().ok())
        else {
            return self.deny("deny_missing_or_unknown_label");
        };
        let own = |c| {
            r.principal
                .memberships
                .iter()
                .filter(move |m| m.circle == c)
        };
        if own(r.resource.circle).any(|m| m.revoked) {
            return self.deny("deny_revoked");
        }
        if r.action == Action::Create
            && r.resource
                .category
                .as_deref()
                .is_some_and(|c| EXCLUDED_CATEGORIES.contains(&c))
        {
            return self.deny("deny_excluded_category");
        }
        if r.action.is_mutating()
            && r.context.origin == crate::Origin::Untrusted
            && !r.context.approved
        {
            return self.deny("deny_unapproved_untrusted_mutation");
        }

        let me = r.principal.id;
        let is_owner = r.resource.owner == Some(me);
        let is_grantee = r.resource.grantees.contains(&me);
        let role = own(r.resource.circle).map(|m| m.role).next();
        let ancestor_owner = r
            .resource
            .ancestors
            .iter()
            .any(|a| own(*a).any(|m| !m.revoked && m.role == Role::Owner));
        let a = r.action;

        let circle_rule = || match role {
            Some(Role::Owner) => Some("allow_circle_owner"),
            Some(Role::Member)
                if matches!(
                    a,
                    Action::Read
                        | Action::Discover
                        | Action::Create
                        | Action::Update
                        | Action::Execute
                ) =>
            {
                Some("allow_circle_member")
            }
            Some(Role::Guest) if is_view(a) && is_grantee => Some("allow_named_guest"),
            _ if is_view(a) && ancestor_owner => Some("allow_ancestor_owner_view"),
            _ => None,
        };

        let verdict = match label {
            Confidentiality::Secret => is_owner.then_some("allow_secret_owner"),
            Confidentiality::Restricted => {
                if is_owner {
                    Some("allow_restricted_owner")
                } else if is_view(a) && is_grantee {
                    Some("allow_restricted_grantee")
                } else {
                    None
                }
            }
            Confidentiality::Circle => circle_rule(),
            Confidentiality::Public => {
                if is_view(a) {
                    Some("allow_public_view")
                } else {
                    circle_rule()
                }
            }
            // Confidentiality is #[non_exhaustive]: a label this code does not know is a deny.
            _ => None,
        };
        verdict.map_or_else(|| self.deny("deny_default"), |name| self.allow(name))
    }
}
