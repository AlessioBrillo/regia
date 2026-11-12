//! The synthetic dataset as the simulator needs it: circles by index, objects, memberships.

use std::collections::{BTreeMap, BTreeSet};

use regia_domain::{
    Confidentiality, Object, PrincipalId,
    synthetic::{Profile, generate},
};

use crate::{Circle, Device};

/// A task, event or document of the dataset.
#[derive(Debug, Clone)]
pub struct Item {
    /// Object id (UUID text).
    pub id: String,
    /// Owning circle.
    pub circle: Circle,
    /// Confidentiality label.
    pub label: Confidentiality,
    /// Title; contains `CANARY-` exactly when the label is `restricted` or `secret`.
    pub title: String,
}

/// Dataset reduced to what sync needs.
#[derive(Debug, Clone)]
pub struct Dataset {
    /// Number of circles.
    pub circles: usize,
    /// All items, in dataset order.
    pub items: Vec<Item>,
    /// Circles of each principal that holds at least one membership.
    pub member_of: Vec<BTreeSet<Circle>>,
}

impl Dataset {
    /// Builds the dataset for `(seed, profile)`.
    #[must_use]
    pub fn load(seed: u64, profile: &Profile) -> Self {
        let mut circle_ix = BTreeMap::new();
        let mut members: BTreeMap<PrincipalId, BTreeSet<Circle>> = BTreeMap::new();
        let mut raw = Vec::new();
        for env in generate(seed, profile) {
            match env.into_object() {
                Object::Circle(c) => {
                    let n = circle_ix.len();
                    circle_ix.insert(c.id, n);
                }
                Object::Membership(m) => raw.push((None, Some(m))),
                Object::Task(t) => raw.push((Some((t.header, t.title.value)), None)),
                Object::Event(e) => raw.push((Some((e.header, e.title.value)), None)),
                Object::Document(d) => raw.push((Some((d.header, d.title.value)), None)),
                _ => {}
            }
        }
        let mut items = Vec::new();
        for (item, membership) in raw {
            if let Some(m) = membership {
                if let Some(&c) = circle_ix.get(&m.circle) {
                    members.entry(m.principal).or_default().insert(c);
                }
            }
            if let Some((h, title)) = item {
                if let Some(&circle) = circle_ix.get(&h.circle) {
                    items.push(Item {
                        id: h.id.as_uuid().to_string(),
                        circle,
                        label: h.confidentiality,
                        title,
                    });
                }
            }
        }
        Self {
            circles: circle_ix.len(),
            items,
            member_of: members.into_values().collect(),
        }
    }

    /// Circles device `d` belongs to: the circles of principal `d % P`.
    #[must_use]
    pub fn circles_of(&self, d: Device) -> BTreeSet<Circle> {
        self.member_of[d as usize % self.member_of.len()].clone()
    }

    /// Devices (of `0..n`) that belong to `circle`.
    #[must_use]
    pub fn members(&self, circle: Circle, n: u32) -> Vec<Device> {
        (0..n)
            .filter(|&d| self.circles_of(d).contains(&circle))
            .collect()
    }

    /// Whether a canary of this item must never reach a device outside its circle.
    #[must_use]
    pub fn is_canary(item: &Item) -> bool {
        item.title.contains("CANARY-")
    }
}

#[cfg(test)]
mod tests {
    use regia_domain::synthetic::SMALL;

    use super::*;

    #[test]
    fn small_dataset_loads_and_has_canaries_and_partial_membership() {
        let d = Dataset::load(42, &SMALL);
        assert_eq!(d.circles, 6);
        assert_eq!(d.items.len(), 36);
        assert!(d.items.iter().any(Dataset::is_canary));
        assert!(
            d.items
                .iter()
                .all(|i| Dataset::is_canary(i) == (i.label >= Confidentiality::Restricted))
        );
        // At least one (device, circle) pair where the device is *not* a member: the leak test needs it.
        assert!((0..4).any(|dev| d.circles_of(dev).len() < d.circles));
    }
}
