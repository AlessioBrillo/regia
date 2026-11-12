//! P-10: random principal/resource/context triples; every engine must agree with the oracle (zero disagreements).
//! Tree: c0 ⊃ c1 ⊃ c2, c0 ⊃ c3, c4 unrelated. Synthetic identities only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::OnceLock;

use proptest::prelude::*;
use regia_domain::{CircleId, PrincipalId, Role};
use regia_spike_pol01::{
    Action, Context, Engine, Member, Origin, Principal, Request, Resource, cedar::CedarEngine,
    oracle::Oracle, rego::RegoEngine,
};

struct Ids {
    circles: [CircleId; 5],
    people: [PrincipalId; 4],
}

fn ids() -> &'static Ids {
    static IDS: OnceLock<Ids> = OnceLock::new();
    IDS.get_or_init(|| Ids {
        circles: std::array::from_fn(|_| CircleId::new()),
        people: std::array::from_fn(|_| PrincipalId::new()),
    })
}

/// Parent chain, nearest first, for circle index `i`.
fn ancestors(i: usize) -> Vec<usize> {
    match i {
        1 | 3 => vec![0],
        2 => vec![1, 0],
        _ => vec![],
    }
}

fn role() -> impl Strategy<Value = Role> {
    prop_oneof![Just(Role::Owner), Just(Role::Member), Just(Role::Guest)]
}

fn request() -> impl Strategy<Value = Request> {
    let membership = proptest::option::of((role(), any::<bool>()));
    let label = proptest::option::of(prop_oneof![
        Just("public"),
        Just("circle"),
        Just("restricted"),
        Just("secret"),
        Just("bogus"),
        Just(""),
    ]);
    let category = proptest::option::of(prop_oneof![
        Just("note"),
        Just("health"),
        Just("finance"),
        Just("id_document")
    ]);
    (
        0..4usize,
        proptest::collection::vec(membership, 5),
        0..8usize,
        0..5usize,
        label,
        proptest::option::of(0..4usize),
        proptest::collection::vec(any::<bool>(), 4),
        category,
        (any::<bool>(), any::<bool>()),
    )
        .prop_map(
            |(who, ms, action, circle, label, owner, grantees, category, (untrusted, approved))| {
                let ids = ids();
                Request {
                    principal: Principal {
                        id: ids.people[who],
                        memberships: ms
                            .into_iter()
                            .enumerate()
                            .filter_map(|(i, m)| {
                                m.map(|(role, revoked)| Member {
                                    circle: ids.circles[i],
                                    role,
                                    revoked,
                                })
                            })
                            .collect(),
                    },
                    action: Action::ALL[action],
                    resource: Resource {
                        circle: ids.circles[circle],
                        ancestors: ancestors(circle)
                            .into_iter()
                            .map(|i| ids.circles[i])
                            .collect(),
                        label: label.map(str::to_owned),
                        owner: owner.map(|o| ids.people[o]),
                        grantees: grantees
                            .into_iter()
                            .enumerate()
                            .filter_map(|(i, g)| g.then_some(ids.people[i]))
                            .collect(),
                        category: category.map(str::to_owned),
                    },
                    context: Context {
                        origin: if untrusted {
                            Origin::Untrusted
                        } else {
                            Origin::User
                        },
                        approved,
                    },
                }
            },
        )
}

thread_local! {
    // Building an engine parses and validates policies; do it once per test thread, not per case.
    static ENGINES: (Oracle, CedarEngine, RegoEngine) = (
        Oracle::new("v1"),
        CedarEngine::new("v1").unwrap(),
        RegoEngine::new("v1").unwrap(),
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    #[test]
    fn engines_agree_with_oracle(r in request()) {
        let (oracle, cedar, rego) = ENGINES.with(|(o, c, g)| (o.decide(&r), c.decide(&r), g.decide(&r)));
        prop_assert_eq!(oracle.allow, cedar.allow, "cedar {:?} vs oracle {:?}", cedar, oracle);
        prop_assert_eq!(oracle.allow, rego.allow, "rego {:?} vs oracle {:?}", rego, oracle);
    }
}
