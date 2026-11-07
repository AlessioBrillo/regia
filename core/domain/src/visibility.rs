//! Server visibility: what the operator's backend may know about an object (report §4.6).
//!
//! The report forbids implicit answers: every object declares whether it is server-readable,
//! end-to-end encrypted, or only transiently processed by an authorised worker.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// Returned when parsing an unrecognised visibility; never defaulted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown server visibility: {0:?}")]
pub struct UnknownVisibility(pub String);

/// What the server side may see of an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ServerVisibility {
    /// The server stores and may interpret the plaintext.
    ServerReadable,
    /// The server only relays ciphertext it cannot interpret.
    EndToEndEncrypted,
    /// Plaintext exists only inside an authorised worker for the duration of a task.
    TransientWorker,
}

impl ServerVisibility {
    /// Stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ServerReadable => "server_readable",
            Self::EndToEndEncrypted => "end_to_end_encrypted",
            Self::TransientWorker => "transient_worker",
        }
    }
}

impl fmt::Display for ServerVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ServerVisibility {
    type Err = UnknownVisibility;

    fn from_str(s: &str) -> Result<Self, UnknownVisibility> {
        match s {
            "server_readable" => Ok(Self::ServerReadable),
            "end_to_end_encrypted" => Ok(Self::EndToEndEncrypted),
            "transient_worker" => Ok(Self::TransientWorker),
            other => Err(UnknownVisibility(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_reject_unknown() {
        for v in [
            ServerVisibility::ServerReadable,
            ServerVisibility::EndToEndEncrypted,
            ServerVisibility::TransientWorker,
        ] {
            assert_eq!(v, v.as_str().parse().unwrap());
            let json = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str(&json).unwrap());
        }
        assert!("plaintext".parse::<ServerVisibility>().is_err());
        assert!(serde_json::from_str::<ServerVisibility>("\"\"").is_err());
    }
}
