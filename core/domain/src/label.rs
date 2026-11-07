//! Confidentiality labels (M8). Names are provisional until the threat model fixes the taxonomy.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// Returned when parsing an unrecognised label. Unknown labels are rejected, never defaulted:
/// falling back to a permissive label would violate deny-by-default.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown confidentiality label: {0:?}")]
pub struct UnknownLabel(pub String);

/// How widely an object may be exposed. Ordered from least to most restrictive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Confidentiality {
    /// Visible beyond the circle (e.g. shared deliberately).
    Public,
    /// Visible to members of the owning circle.
    Circle,
    /// Visible only to explicitly named principals.
    Restricted,
    /// Visible only to the owner; never sent to a cloud model or relay in the clear.
    Secret,
}

impl Confidentiality {
    /// Stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Circle => "circle",
            Self::Restricted => "restricted",
            Self::Secret => "secret",
        }
    }
}

impl fmt::Display for Confidentiality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Confidentiality {
    type Err = UnknownLabel;

    fn from_str(s: &str) -> Result<Self, UnknownLabel> {
        match s {
            "public" => Ok(Self::Public),
            "circle" => Ok(Self::Circle),
            "restricted" => Ok(Self::Restricted),
            "secret" => Ok(Self::Secret),
            other => Err(UnknownLabel(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Confidentiality; 4] = [
        Confidentiality::Public,
        Confidentiality::Circle,
        Confidentiality::Restricted,
        Confidentiality::Secret,
    ];

    #[test]
    fn ordered_least_to_most_restrictive() {
        assert!(ALL.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn str_and_json_roundtrip() {
        for l in ALL {
            assert_eq!(l, l.as_str().parse().unwrap());
            let json = serde_json::to_string(&l).unwrap();
            assert_eq!(json, format!("\"{}\"", l.as_str()));
            assert_eq!(l, serde_json::from_str(&json).unwrap());
        }
    }

    #[test]
    fn unknown_is_rejected_not_defaulted() {
        assert!("Public".parse::<Confidentiality>().is_err());
        assert!("".parse::<Confidentiality>().is_err());
        assert!(serde_json::from_str::<Confidentiality>("\"top_secret\"").is_err());
    }
}
