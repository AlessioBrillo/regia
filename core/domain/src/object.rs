//! The metadata every domain object must carry. All fields are mandatory: there is no
//! `Default`, so a label, a server-visibility declaration and provenance can never be implied.

use serde::{Deserialize, Serialize};

use crate::{
    id::{CircleId, ObjectId},
    label::Confidentiality,
    provenance::Provenance,
    visibility::ServerVisibility,
};

/// Mandatory header of every object (AR-04, AR-06, report §4.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectHeader {
    /// Stable identity.
    pub id: ObjectId,
    /// Owning circle.
    pub circle: CircleId,
    /// Confidentiality label.
    pub confidentiality: Confidentiality,
    /// What the server side may see.
    pub server_visibility: ServerVisibility,
    /// Where the object came from.
    pub provenance: Provenance,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::UnsourcedReason;

    fn header() -> ObjectHeader {
        ObjectHeader {
            id: ObjectId::new(),
            circle: CircleId::new(),
            confidentiality: Confidentiality::Circle,
            server_visibility: ServerVisibility::EndToEndEncrypted,
            provenance: Provenance::Unsourced {
                reason: UnsourcedReason::new("typed by user").unwrap(),
            },
        }
    }

    #[test]
    fn json_roundtrip() {
        let h = header();
        let json = serde_json::to_string(&h).unwrap();
        assert_eq!(h, serde_json::from_str(&json).unwrap());
    }

    #[test]
    fn every_field_is_required() {
        let mut v: serde_json::Value = serde_json::to_value(header()).unwrap();
        for field in [
            "id",
            "circle",
            "confidentiality",
            "server_visibility",
            "provenance",
        ] {
            let removed = v.as_object_mut().unwrap().remove(field).unwrap();
            assert!(
                serde_json::from_value::<ObjectHeader>(v.clone()).is_err(),
                "{field}"
            );
            v[field] = removed;
        }
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut v = serde_json::to_value(header()).unwrap();
        v["extra"] = serde_json::json!(1);
        assert!(serde_json::from_value::<ObjectHeader>(v).is_err());
    }
}
