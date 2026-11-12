//! POL-01 case table P-01..P-09, P-11. Every row runs against every engine in `engines()`.
//! Synthetic identities only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use regia_domain::{CircleId, PrincipalId, Role};
use regia_spike_pol01::{
    Action, Context, Engine, Member, Origin, Principal, Request, Resource, engines, oracle::Oracle,
};

struct World {
    root: CircleId,
    child: CircleId,
    sibling: CircleId,
    alice: PrincipalId, // Owner of root
    bob: PrincipalId,   // Member of child
    carol: PrincipalId, // Member of sibling
    dave: PrincipalId,  // Member of root
}

#[allow(clippy::unused_self)] // keeps call sites uniform: w.who(..), w.obj(..)
impl World {
    fn new() -> Self {
        Self {
            root: CircleId::new(),
            child: CircleId::new(),
            sibling: CircleId::new(),
            alice: PrincipalId::new(),
            bob: PrincipalId::new(),
            carol: PrincipalId::new(),
            dave: PrincipalId::new(),
        }
    }

    fn who(&self, id: PrincipalId, circle: CircleId, role: Role, revoked: bool) -> Principal {
        Principal {
            id,
            memberships: vec![Member {
                circle,
                role,
                revoked,
            }],
        }
    }

    /// An object in `circle` (nested under `ancestors`) with the given label.
    fn obj(&self, circle: CircleId, ancestors: &[CircleId], label: Option<&str>) -> Resource {
        Resource {
            circle,
            ancestors: ancestors.to_vec(),
            label: label.map(str::to_owned),
            owner: Some(self.alice),
            grantees: vec![],
            category: None,
        }
    }
}

fn user() -> Context {
    Context {
        origin: Origin::User,
        approved: false,
    }
}

fn req(principal: Principal, action: Action, resource: Resource, context: Context) -> Request {
    Request {
        principal,
        action,
        resource,
        context,
    }
}

/// Asserts the expected outcome on every engine and that a reason is always given.
fn check(name: &str, request: &Request, expect_allow: bool) {
    for e in engines() {
        let d = e.decide(request);
        assert_eq!(d.allow, expect_allow, "{name} on {}: {d:?}", e.name());
        assert!(
            !d.reasons.is_empty(),
            "{name} on {}: no reason given",
            e.name()
        );
        assert!(
            !d.policy_version.is_empty(),
            "{name} on {}: no version",
            e.name()
        );
    }
}

#[test]
fn p01_member_reads_own_circle() {
    let w = World::new();
    let r = w.obj(w.child, &[w.root], Some("circle"));
    check(
        "P-01",
        &req(
            w.who(w.bob, w.child, Role::Member, false),
            Action::Read,
            r,
            user(),
        ),
        true,
    );
}

#[test]
fn p02_sibling_and_parent_are_denied() {
    let w = World::new();
    let child_obj = w.obj(w.child, &[w.root], Some("circle"));
    check(
        "P-02 sibling",
        &req(
            w.who(w.carol, w.sibling, Role::Member, false),
            Action::Read,
            child_obj,
            user(),
        ),
        false,
    );
    // child member reads an object of the parent circle
    let root_obj = w.obj(w.root, &[], Some("circle"));
    check(
        "P-02 child→parent",
        &req(
            w.who(w.bob, w.child, Role::Member, false),
            Action::Read,
            root_obj,
            user(),
        ),
        false,
    );
}

#[test]
fn p03_nested_circles_parent_owner_descends_view_only() {
    let w = World::new();
    let child_obj = w.obj(w.child, &[w.root], Some("circle"));
    let owner = w.who(w.alice, w.root, Role::Owner, false);
    check(
        "P-03 owner read",
        &req(owner.clone(), Action::Read, child_obj.clone(), user()),
        true,
    );
    check(
        "P-03 owner discover",
        &req(owner.clone(), Action::Discover, child_obj.clone(), user()),
        true,
    );
    // view only: no mutation through inheritance
    check(
        "P-03 owner update",
        &req(owner.clone(), Action::Update, child_obj, user()),
        false,
    );
    // a parent *Member* gets nothing
    let child_obj = w.obj(w.child, &[w.root], Some("circle"));
    check(
        "P-03 member",
        &req(
            w.who(w.dave, w.root, Role::Member, false),
            Action::Read,
            child_obj,
            user(),
        ),
        false,
    );
    // restricted/secret are never inherited
    for label in ["restricted", "secret"] {
        let mut o = w.obj(w.child, &[w.root], Some(label));
        o.owner = Some(w.bob);
        check(
            &format!("P-03 {label} not inherited"),
            &req(owner.clone(), Action::Read, o, user()),
            false,
        );
    }
}

#[test]
fn p04_restricted_and_secret_only_named() {
    let w = World::new();
    let mut restricted = w.obj(w.root, &[], Some("restricted"));
    restricted.grantees = vec![w.dave];
    let dave = w.who(w.dave, w.root, Role::Member, false);
    check(
        "P-04 grantee read",
        &req(dave.clone(), Action::Read, restricted.clone(), user()),
        true,
    );
    check(
        "P-04 grantee update",
        &req(dave, Action::Update, restricted.clone(), user()),
        false,
    );
    let carol = w.who(w.carol, w.root, Role::Member, false);
    check(
        "P-04 member not named",
        &req(carol.clone(), Action::Read, restricted, user()),
        false,
    );
    let secret = w.obj(w.root, &[], Some("secret"));
    check(
        "P-04 secret owner",
        &req(
            w.who(w.alice, w.root, Role::Owner, false),
            Action::Read,
            secret.clone(),
            user(),
        ),
        true,
    );
    check(
        "P-04 secret circle owner-role non-owner",
        &req(
            w.who(w.dave, w.root, Role::Owner, false),
            Action::Read,
            secret,
            user(),
        ),
        false,
    );
}

#[test]
fn p05_discover_is_separate_from_read_and_does_not_leak() {
    let w = World::new();
    for label in ["restricted", "secret"] {
        let o = w.obj(w.root, &[], Some(label));
        let carol = w.who(w.carol, w.root, Role::Member, false);
        check(
            &format!("P-05 {label} discover"),
            &req(carol, Action::Discover, o, user()),
            false,
        );
    }
    // a member may discover a circle object they can read
    let o = w.obj(w.root, &[], Some("circle"));
    check(
        "P-05 circle discover",
        &req(
            w.who(w.dave, w.root, Role::Member, false),
            Action::Discover,
            o,
            user(),
        ),
        true,
    );
}

#[test]
fn p06_missing_unknown_or_invalid_is_deny() {
    let w = World::new();
    let p = w.who(w.alice, w.root, Role::Owner, false);
    for (name, label) in [
        ("missing", None),
        ("unknown", Some("top_secret")),
        ("empty", Some("")),
    ] {
        let o = w.obj(w.root, &[], label);
        check(
            &format!("P-06 label {name}"),
            &req(p.clone(), Action::Read, o, user()),
            false,
        );
    }
    // a principal with no membership at all
    let nobody = Principal {
        id: PrincipalId::new(),
        memberships: vec![],
    };
    check(
        "P-06 no memberships",
        &req(
            nobody,
            Action::Read,
            w.obj(w.root, &[], Some("circle")),
            user(),
        ),
        false,
    );
}

#[test]
fn p07_untrusted_mutation_needs_approval() {
    let w = World::new();
    let p = w.who(w.alice, w.root, Role::Owner, false);
    let o = w.obj(w.root, &[], Some("circle"));
    let untrusted = Context {
        origin: Origin::Untrusted,
        approved: false,
    };
    let approved = Context {
        origin: Origin::Untrusted,
        approved: true,
    };
    for a in Action::ALL.into_iter().filter(|a| a.is_mutating()) {
        check(
            &format!("P-07 {} unapproved", a.as_str()),
            &req(p.clone(), a, o.clone(), untrusted),
            false,
        );
        check(
            &format!("P-07 {} approved", a.as_str()),
            &req(p.clone(), a, o.clone(), approved),
            true,
        );
    }
    // reading from untrusted origin is not a mutation
    check("P-07 read", &req(p, Action::Read, o, untrusted), true);
}

#[test]
fn p08_decision_is_bound_to_policy_version() {
    let w = World::new();
    let p = w.who(w.alice, w.root, Role::Owner, false);
    let rq = req(p, Action::Read, w.obj(w.root, &[], Some("circle")), user());
    let (v1, v2) = (Oracle::new("v1"), Oracle::new("v2"));
    assert_eq!(v1.decide(&rq).policy_version, "v1");
    assert_eq!(v2.decide(&rq).policy_version, "v2");
}

#[test]
fn p09_revoked_member_is_denied_everything() {
    let w = World::new();
    let revoked = w.who(w.alice, w.root, Role::Owner, true);
    for label in ["public", "circle", "restricted", "secret"] {
        for a in Action::ALL {
            let o = w.obj(w.root, &[], Some(label));
            check(
                &format!("P-09 {label} {}", a.as_str()),
                &req(revoked.clone(), a, o, user()),
                false,
            );
        }
    }
}

#[test]
fn p11_excluded_categories_cannot_be_created() {
    let w = World::new();
    let p = w.who(w.alice, w.root, Role::Owner, false);
    for c in ["health", "minors", "finance", "credentials", "id_document"] {
        let mut o = w.obj(w.root, &[], Some("circle"));
        o.category = Some(c.to_owned());
        check(
            &format!("P-11 {c}"),
            &req(p.clone(), Action::Create, o, user()),
            false,
        );
    }
    let mut ok = w.obj(w.root, &[], Some("circle"));
    ok.category = Some("note".to_owned());
    check("P-11 ordinary", &req(p, Action::Create, ok, user()), true);
}

#[test]
fn p06_malformed_policy_yields_no_engine() {
    use regia_spike_pol01::cedar::CedarEngine;
    assert!(
        CedarEngine::from_sources("entity User;", "permit (principal, action, resource", "bad")
            .is_err()
    );
    // A policy that fails strict validation (unknown attribute) is refused too.
    let schema = include_str!("../policies/regia.cedarschema");
    let bad = r#"permit (principal, action, resource) when { resource.nonexistent == "x" };"#;
    assert!(CedarEngine::from_sources(schema, bad, "bad").is_err());
}

#[test]
fn p06_malformed_rego_yields_no_engine() {
    use regia_spike_pol01::rego::RegoEngine;
    assert!(RegoEngine::from_source("package x\nallow if {", "bad").is_err());
}
