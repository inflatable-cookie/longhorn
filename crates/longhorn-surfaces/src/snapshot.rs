use longhorn_core::SurfaceRevision;
use serde::{Deserialize, Serialize};

use crate::{SURFACE_PROTOCOL_VERSION, SurfaceDocument};

/// Host-issued epoch for one Surface protocol authority.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct SurfaceProtocolEpoch(u64);

impl SurfaceProtocolEpoch {
    /// Constructs an epoch from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Complete current Surface authority returned by a snapshot query.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(try_from = "UncheckedSurfaceSnapshot")]
#[serde(deny_unknown_fields)]
pub struct SurfaceSnapshot {
    protocol_version: u32,
    epoch: SurfaceProtocolEpoch,
    revision: SurfaceRevision,
    document: SurfaceDocument,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedSurfaceSnapshot {
    protocol_version: u32,
    epoch: SurfaceProtocolEpoch,
    revision: SurfaceRevision,
    document: SurfaceDocument,
}

impl TryFrom<UncheckedSurfaceSnapshot> for SurfaceSnapshot {
    type Error = String;

    fn try_from(value: UncheckedSurfaceSnapshot) -> Result<Self, Self::Error> {
        if value.protocol_version != SURFACE_PROTOCOL_VERSION {
            return Err(format!(
                "Surface protocol version {} is unsupported; expected {SURFACE_PROTOCOL_VERSION}",
                value.protocol_version
            ));
        }
        if value.revision != value.document.revision() {
            return Err(format!(
                "Surface snapshot revision {} does not match document revision {}",
                value.revision.get(),
                value.document.revision().get()
            ));
        }
        Ok(Self {
            protocol_version: value.protocol_version,
            epoch: value.epoch,
            revision: value.revision,
            document: value.document,
        })
    }
}

impl SurfaceSnapshot {
    /// Constructs one snapshot and copies its revision from the document.
    #[must_use]
    pub fn new(epoch: SurfaceProtocolEpoch, document: SurfaceDocument) -> Self {
        let revision = document.revision();
        Self {
            protocol_version: SURFACE_PROTOCOL_VERSION,
            epoch,
            revision,
            document,
        }
    }

    /// Returns the exact Surface protocol version.
    #[must_use]
    pub const fn protocol_version(&self) -> u32 {
        self.protocol_version
    }

    /// Returns the current host authority epoch.
    #[must_use]
    pub const fn epoch(&self) -> SurfaceProtocolEpoch {
        self.epoch
    }

    /// Returns the authoritative durable revision.
    #[must_use]
    pub const fn revision(&self) -> SurfaceRevision {
        self.revision
    }

    /// Returns the complete current document.
    #[must_use]
    pub const fn document(&self) -> &SurfaceDocument {
        &self.document
    }
}

/// Surface invalidation hint emitted after one authoritative mutation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct SurfaceChangedEvent {
    protocol_version: u32,
    epoch: SurfaceProtocolEpoch,
    revision: SurfaceRevision,
}

impl SurfaceChangedEvent {
    /// Constructs one invalidation hint.
    #[must_use]
    pub const fn new(epoch: SurfaceProtocolEpoch, revision: SurfaceRevision) -> Self {
        Self {
            protocol_version: SURFACE_PROTOCOL_VERSION,
            epoch,
            revision,
        }
    }

    /// Returns the exact Surface protocol version.
    #[must_use]
    pub const fn protocol_version(self) -> u32 {
        self.protocol_version
    }

    /// Returns the current host authority epoch.
    #[must_use]
    pub const fn epoch(self) -> SurfaceProtocolEpoch {
        self.epoch
    }

    /// Returns the authoritative durable revision.
    #[must_use]
    pub const fn revision(self) -> SurfaceRevision {
        self.revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_deserialization_rejects_future_and_mismatched_authority() {
        let document = SurfaceDocument::new(SurfaceRevision::new(3), [], [], []);
        let snapshot = SurfaceSnapshot::new(SurfaceProtocolEpoch::new(2), document);
        let value = serde_json::to_value(snapshot).unwrap();

        assert!(serde_json::from_value::<SurfaceSnapshot>(value.clone()).is_ok());

        let mut future = value.clone();
        future["protocol_version"] = serde_json::json!(2);
        assert!(serde_json::from_value::<SurfaceSnapshot>(future).is_err());

        let mut mismatched = value;
        mismatched["revision"] = serde_json::json!(4);
        assert!(serde_json::from_value::<SurfaceSnapshot>(mismatched).is_err());
    }
}
