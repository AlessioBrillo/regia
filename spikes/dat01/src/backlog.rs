//! S-08: five clients write offline for a long time, then all reconnect.
//!
//! Frozen rules (experiment card, increment 3): per client `ops` offline operations; pass =
//! convergence time p95 < 5 s (in-process, a lower bound), bytes <= 10x the net payload, no
//! silent loss, canary scan clean. The thresholds live in the example, not here.

use std::{collections::BTreeMap, collections::BTreeSet, time::Instant};

use crate::{Error, Op, Replica, sim::Sim};

/// Clients in the scenario.
pub const CLIENTS: u32 = 5;
const CANARY: &str = "CANARY-1-0";
const SHARED: &str = "shared";

/// One run's measurements.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Operations written offline across all clients.
    pub ops: usize,
    /// Net payload of those operations (ids, field names and values), bytes.
    pub payload: usize,
    /// Bytes that crossed the wire during the reconnect.
    pub bytes: usize,
    /// `sync_all` after reconnect, milliseconds of in-process CPU time.
    pub sync_ms: f64,
    /// Every replica shows the same circle.
    pub converged: bool,
    /// Writes whose last value is missing, wrong, or neither shown nor listed as a conflict.
    pub lost: usize,
    /// The canary reached a device outside its circle.
    pub leaked: bool,
}

/// Small deterministic generator so a seed fixes the whole run.
fn next(s: &mut u64) -> u64 {
    *s = s
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *s >> 33
}

/// Run S-08 on alternative `R`: each client creates `ops / 10` own objects, then edits them (and
/// the shared object every tenth op) until it has made `ops` operations, all while offline.
#[allow(clippy::too_many_lines)] // one linear scenario
pub fn run<R: Replica>(seed: u64, ops: usize) -> Result<Outcome, Error> {
    let mut scope = vec![BTreeSet::from([0]); CLIENTS as usize];
    scope[CLIENTS as usize - 1].insert(1); // the last device alone holds circle 1
    let mut sim = Sim::<R>::new(scope);
    for c in &mut sim.clients {
        c.set_schema(crate::Schema::V1);
    }
    let fields = |t: &str| BTreeMap::from([("title".to_owned(), t.to_owned())]);
    let (mut payload, mut rng) = (0, seed);
    let apply = |sim: &mut Sim<R>, d, c, op: Op, payload: &mut usize| {
        *payload += match &op {
            Op::Create { obj, fields } => {
                obj.len() + fields.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>()
            }
            Op::Set { obj, field, value } => obj.len() + field.len() + value.len(),
            _ => 0,
        };
        sim.op(d, c, op)
    };

    let op = Op::Create {
        obj: SHARED.into(),
        fields: fields("shared"),
    };
    apply(&mut sim, 0, 0, op, &mut payload)?;
    sim.op(
        CLIENTS - 1,
        1,
        Op::Create {
            obj: "c1".into(),
            fields: fields(CANARY),
        },
    )?;
    sim.sync_all()?;

    for d in 0..CLIENTS {
        sim.set_online(d, false);
    }
    let own = (ops / 10).max(1);
    // Last value each device wrote to each field: it must be shown or listed as a conflict.
    let mut written: BTreeMap<(String, String, u32), String> = BTreeMap::new();
    for d in 0..CLIENTS {
        for i in 0..own {
            let obj = format!("d{d}-o{i}");
            let op = Op::Create {
                obj: obj.clone(),
                fields: fields(&format!("t{d}-{i}")),
            };
            apply(&mut sim, d, 0, op, &mut payload)?;
            written.insert((obj, "title".into(), d), format!("t{d}-{i}"));
        }
        for k in own..ops {
            let (obj, field) = if k % 10 == 0 {
                (SHARED.to_owned(), "f".to_owned())
            } else {
                let i = usize::try_from(next(&mut rng)).unwrap_or(0) % own;
                (format!("d{d}-o{i}"), "status".to_owned())
            };
            let value = format!("v{d}-{k}");
            let op = Op::Set {
                obj: obj.clone(),
                field: field.clone(),
                value: value.clone(),
            };
            apply(&mut sim, d, 0, op, &mut payload)?;
            written.insert((obj, field, d), value);
        }
    }

    for d in 0..CLIENTS {
        sim.set_online(d, true);
    }
    let before = sim.bytes;
    let t = Instant::now();
    sim.sync_all()?;
    let sync_ms = t.elapsed().as_secs_f64() * 1000.0;

    let snaps: Vec<_> = (0..CLIENTS)
        .map(|d| sim.clients[d as usize].snapshot(0))
        .collect();
    let converged = snaps.windows(2).all(|w| w[0] == w[1]);
    let lost = written
        .iter()
        .filter(|((obj, field, _), value)| {
            snaps[0]
                .objects
                .get(obj)
                .and_then(|o| o.fields.get(field))
                .is_none_or(|shown| {
                    shown != *value
                        && !snaps[0]
                            .conflicts
                            .iter()
                            .any(|c| &c.obj == obj && &c.field == field && &c.lost == *value)
                })
        })
        .count();
    let leaked = (0..CLIENTS - 1).any(|d| sim.clients[d as usize].dump_text().contains(CANARY));
    Ok(Outcome {
        ops: ops * CLIENTS as usize,
        payload,
        bytes: sim.bytes - before,
        sync_ms,
        converged,
        lost,
        leaked,
    })
}
