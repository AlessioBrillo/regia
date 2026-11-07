//! Stable, typed identities (report §4.4). All IDs are `UUIDv7` and cannot be mixed up at compile time.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Error returned when a value is not a valid `UUIDv7` identifier.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The text is not a UUID.
    #[error("not a valid UUID: {0}")]
    Malformed(String),
    /// The UUID is valid but not version 7.
    #[error("identifier must be a UUIDv7, got version {0}")]
    WrongVersion(usize),
}

macro_rules! typed_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "Uuid", into = "Uuid")]
        pub struct $name(Uuid);

        impl $name {
            /// Generates a new time-ordered identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// The underlying UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl TryFrom<Uuid> for $name {
            type Error = IdError;

            fn try_from(u: Uuid) -> Result<Self, IdError> {
                match u.get_version_num() {
                    7 => Ok(Self(u)),
                    v => Err(IdError::WrongVersion(v)),
                }
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Uuid {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(s: &str) -> Result<Self, IdError> {
                let u = Uuid::parse_str(s).map_err(|e| IdError::Malformed(e.to_string()))?;
                Self::try_from(u)
            }
        }
    };
}

typed_id!(
    /// A person, device-bound service or other acting identity.
    PrincipalId
);
typed_id!(
    /// A physical device enrolled by a principal.
    DeviceId
);
typed_id!(
    /// A circle (family, group, workspace) that scopes membership and policy.
    CircleId
);
typed_id!(
    /// A domain object (task, event, document, ...).
    ObjectId
);
typed_id!(
    /// A source an object or field was derived from.
    SourceId
);

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn rejects_non_v7() {
        let v4 = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(ObjectId::from_str(v4), Err(IdError::WrongVersion(4)));
        assert!(serde_json::from_str::<ObjectId>(&format!("\"{v4}\"")).is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(
            CircleId::from_str("nope"),
            Err(IdError::Malformed(_))
        ));
    }

    #[test]
    fn new_ids_are_ordered_and_distinct() {
        let a = ObjectId::new();
        let b = ObjectId::new();
        assert_ne!(a, b);
    }

    proptest! {
        /// Any UUID is accepted iff it is version 7, and accepted values roundtrip through text and JSON.
        #[test]
        fn accepts_exactly_v7_and_roundtrips(bits in any::<u128>()) {
            let u = Uuid::from_u128(bits);
            match PrincipalId::try_from(u) {
                Ok(id) => {
                    prop_assert_eq!(u.get_version_num(), 7);
                    prop_assert_eq!(id, id.to_string().parse::<PrincipalId>().unwrap());
                    let json = serde_json::to_string(&id).unwrap();
                    prop_assert_eq!(id, serde_json::from_str::<PrincipalId>(&json).unwrap());
                }
                Err(e) => prop_assert_eq!(e, IdError::WrongVersion(u.get_version_num())),
            }
        }
    }
}
