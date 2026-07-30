use longhorn_core::{BridgeId, DomainId};
use serde::{Deserialize, Serialize};

use crate::{BridgeNegotiationError, BridgeNegotiationErrorCode, BridgeProtocolVersion};

use super::validation::{validate_limit, validate_unique};

/// Maximum domains one hello request may name.
pub const MAXIMUM_REQUESTED_DOMAINS: usize = 256;

/// Checked exact-version hello request from one bridge client.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    try_from = "RawBridgeHelloRequest"
)]
pub struct BridgeHelloRequest {
    protocol_version: BridgeProtocolVersion,
    bridge_id: BridgeId,
    requested_domains: Vec<DomainId>,
}

impl BridgeHelloRequest {
    /// Constructs a bounded hello request for explicitly requested domains.
    pub fn new(
        bridge_id: BridgeId,
        requested_domains: Vec<DomainId>,
    ) -> Result<Self, BridgeNegotiationError> {
        validate_limit(
            requested_domains.len(),
            MAXIMUM_REQUESTED_DOMAINS,
            "requested domains",
        )?;
        validate_unique(
            &requested_domains,
            BridgeNegotiationErrorCode::DuplicateRequestedDomain,
            "requested domain",
        )?;

        Ok(Self {
            protocol_version: BridgeProtocolVersion::CURRENT,
            bridge_id,
            requested_domains,
        })
    }

    /// Returns the exact protocol version.
    #[must_use]
    pub const fn protocol_version(&self) -> BridgeProtocolVersion {
        self.protocol_version
    }

    /// Returns the requesting bridge identity.
    #[must_use]
    pub const fn bridge_id(&self) -> &BridgeId {
        &self.bridge_id
    }

    /// Returns the explicit domain request set.
    #[must_use]
    pub fn requested_domains(&self) -> &[DomainId] {
        &self.requested_domains
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawBridgeHelloRequest {
    protocol_version: BridgeProtocolVersion,
    bridge_id: BridgeId,
    requested_domains: Vec<DomainId>,
}

impl TryFrom<RawBridgeHelloRequest> for BridgeHelloRequest {
    type Error = BridgeNegotiationError;

    fn try_from(raw: RawBridgeHelloRequest) -> Result<Self, Self::Error> {
        let request = Self::new(raw.bridge_id, raw.requested_domains)?;
        debug_assert_eq!(raw.protocol_version, request.protocol_version);
        Ok(request)
    }
}
