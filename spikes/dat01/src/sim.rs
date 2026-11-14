//! Deterministic simulator: N clients, scope-filtered pairwise sync, a global simulated clock.
//!
//! The "relay" is the scope rule: a session between two clients exists only for circles both
//! belong to, so a client never receives bytes of a circle it is not in (AR-04, S-10 precursor).
//! Time never comes from the wall clock; `bytes` and `rounds` count what crossed the wire.

use std::collections::BTreeSet;

use crate::{Circle, Device, Error, Op, Replica};

/// Upper bound on message exchanges per session: a session that does not settle is a bug.
const MAX_ROUNDS: usize = 64;

/// A set of replicas and the network between them.
pub struct Sim<R: Replica> {
    /// The replicas, index = device id.
    pub clients: Vec<R>,
    scope: Vec<BTreeSet<Circle>>,
    online: Vec<bool>,
    /// Simulated milliseconds; strictly increases with every op.
    pub now: u64,
    /// Bytes exchanged by all sessions so far.
    pub bytes: usize,
    /// Message exchanges (one message = one round) so far.
    pub messages: usize,
}

impl<R: Replica> Sim<R> {
    /// `scope[d]` = circles of device `d`; all clients start online.
    #[must_use]
    pub fn new(scope: Vec<BTreeSet<Circle>>) -> Self {
        let clients = scope
            .iter()
            .enumerate()
            .map(|(d, s)| {
                R::new(
                    u32::try_from(d).unwrap_or(u32::MAX),
                    &s.iter().copied().collect::<Vec<_>>(),
                )
            })
            .collect();
        Self {
            online: vec![true; scope.len()],
            clients,
            scope,
            now: 1_000,
            bytes: 0,
            messages: 0,
        }
    }

    /// Takes a device off the network (it keeps writing locally) or brings it back.
    pub fn set_online(&mut self, d: Device, up: bool) {
        self.online[d as usize] = up;
    }

    /// Local op by device `d` one simulated millisecond after the previous op.
    pub fn op(&mut self, d: Device, circle: Circle, op: Op) -> Result<(), Error> {
        self.now += 1;
        self.op_at(d, circle, op, self.now)
    }

    /// Local op at an explicit time (to force equal stamps).
    pub fn op_at(&mut self, d: Device, circle: Circle, op: Op, now_ms: u64) -> Result<(), Error> {
        if !self.scope[d as usize].contains(&circle) {
            return Err(format!("device {d} is not in circle {circle}"));
        }
        self.clients[d as usize].apply_local(circle, op, now_ms)
    }

    /// One sync session between `a` and `b` over every circle they share. Returns whether either
    /// side changed. Offline devices do nothing.
    pub fn sync_pair(&mut self, a: Device, b: Device) -> Result<bool, Error> {
        if a == b || !self.online[a as usize] || !self.online[b as usize] {
            return Ok(false);
        }
        self.clients[a as usize].set_now(self.now);
        self.clients[b as usize].set_now(self.now);
        let shared: Vec<Circle> = self.scope[a as usize]
            .intersection(&self.scope[b as usize])
            .copied()
            .collect();
        let mut changed = false;
        for circle in shared {
            changed |= self.session(a, b, circle)?;
        }
        Ok(changed)
    }

    fn session(&mut self, a: Device, b: Device, circle: Circle) -> Result<bool, Error> {
        let (ai, bi) = (a as usize, b as usize);
        let (mut sa, mut sb) = (R::Sync::default(), R::Sync::default());
        let mut changed = false;
        for _ in 0..MAX_ROUNDS {
            let ma = self.clients[ai].generate(circle, &mut sa);
            let mb = self.clients[bi].generate(circle, &mut sb);
            if ma.is_none() && mb.is_none() {
                return Ok(changed);
            }
            if let Some(m) = ma {
                self.bytes += m.len();
                self.messages += 1;
                changed |= self.clients[bi].receive(circle, &mut sb, &m)?;
            }
            if let Some(m) = mb {
                self.bytes += m.len();
                self.messages += 1;
                changed |= self.clients[ai].receive(circle, &mut sa, &m)?;
            }
        }
        Err(format!(
            "session {a}<->{b} on circle {circle} did not settle"
        ))
    }

    /// Full-mesh sync of every online pair until a whole pass changes nothing.
    pub fn sync_all(&mut self) -> Result<(), Error> {
        let n = u32::try_from(self.clients.len()).map_err(|e| e.to_string())?;
        for _ in 0..MAX_ROUNDS {
            let mut changed = false;
            for a in 0..n {
                for b in (a + 1)..n {
                    changed |= self.sync_pair(a, b)?;
                }
            }
            if !changed {
                return Ok(());
            }
        }
        Err("sync_all did not converge".into())
    }

    /// Devices (of this simulation) that belong to `circle`.
    #[must_use]
    pub fn members(&self, circle: Circle) -> Vec<Device> {
        (0..self.clients.len())
            .filter(|&d| self.scope[d].contains(&circle))
            .map(|d| u32::try_from(d).unwrap_or(u32::MAX))
            .collect()
    }

    /// Circles of device `d`.
    #[must_use]
    pub fn scope_of(&self, d: Device) -> &BTreeSet<Circle> {
        &self.scope[d as usize]
    }
}
