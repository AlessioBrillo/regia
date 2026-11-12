//! POL-01 latency probe: p50/p95 per engine over a mixed decision workload.
//! `cargo run -p regia-spike-pol01 --example latency --release -- [iterations]`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::time::Instant;

use regia_domain::{CircleId, PrincipalId, Role};
use regia_spike_pol01::{Action, Context, Member, Origin, Principal, Request, Resource, engines};

fn workload() -> Vec<Request> {
    let (root, child) = (CircleId::new(), CircleId::new());
    let (alice, bob) = (PrincipalId::new(), PrincipalId::new());
    let mut out = vec![];
    for label in ["public", "circle", "restricted", "secret"] {
        for action in Action::ALL {
            for (who, circle, role) in [(alice, root, Role::Owner), (bob, child, Role::Member)] {
                out.push(Request {
                    principal: Principal {
                        id: who,
                        memberships: vec![Member {
                            circle,
                            role,
                            revoked: false,
                        }],
                    },
                    action,
                    resource: Resource {
                        circle: child,
                        ancestors: vec![root],
                        label: Some(label.to_owned()),
                        owner: Some(alice),
                        grantees: vec![bob],
                        category: None,
                    },
                    context: Context {
                        origin: Origin::User,
                        approved: false,
                    },
                });
            }
        }
    }
    out
}

fn main() {
    let rounds: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(300);
    let load = workload();
    println!("engine,decisions,p50_us,p95_us,max_us");
    for e in engines() {
        let mut us: Vec<f64> = Vec::with_capacity(rounds * load.len());
        for _ in 0..rounds {
            for r in &load {
                let t = Instant::now();
                std::hint::black_box(e.decide(r));
                us.push(t.elapsed().as_secs_f64() * 1e6);
            }
        }
        us.sort_by(f64::total_cmp);
        let q = |pct: usize| us[(us.len() - 1) * pct / 100];
        println!(
            "{},{},{:.1},{:.1},{:.1}",
            e.name(),
            us.len(),
            q(50),
            q(95),
            us[us.len() - 1]
        );
    }
}
