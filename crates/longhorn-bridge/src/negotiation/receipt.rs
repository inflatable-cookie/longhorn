use std::collections::HashSet;

use longhorn_core::{BridgeSessionId, DomainId, TransportFeatureId};
use serde::{Deserialize, Serialize};

use crate::{
    AuthenticationPosture, BridgeConnectionState, BridgeConnectionStatus, BridgeHostDescriptor,
    BridgeNegotiationError, BridgeNegotiationErrorCode, BridgeProtocolVersion,
    DomainAuthorityDescriptor, WriteAuthority,
};

use super::{
    BridgeDiagnostic, BridgeHelloRequest, DomainCapabilityDescriptor,
    validation::{validate_limit, validate_unique, validate_unique_by},
};

/// Maximum transport features one receipt may advertise.
pub const MAXIMUM_TRANSPORT_FEATURES: usize = 128;
/// Maximum domain capability descriptors one receipt may advertise.
pub const MAXIMUM_CAPABILITY_DOMAINS: usize = 256;
/// Maximum domain authority descriptors one receipt may advertise.
pub const MAXIMUM_AUTHORITY_DOMAINS: usize = 256;
/// Maximum diagnostics one receipt may carry.
pub const MAXIMUM_DIAGNOSTICS: usize = 64;

/// Exact-version, checked response describing one negotiated bridge session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    try_from = "RawBridgeNegotiationReceipt"
)]
pub struct BridgeNegotiationReceipt {
    protocol_version: BridgeProtocolVersion,
    host: BridgeHostDescriptor,
    session_id: BridgeSessionId,
    connection: BridgeConnectionStatus,
    authentication: AuthenticationPosture,
    transport_features: Vec<TransportFeatureId>,
    domain_capabilities: Vec<DomainCapabilityDescriptor>,
    domain_authorities: Vec<DomainAuthorityDescriptor>,
    diagnostics: Vec<BridgeDiagnostic>,
}

impl BridgeNegotiationReceipt {
    /// Constructs a bounded receipt and validates internal capability and authority facts.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: BridgeHostDescriptor,
        session_id: BridgeSessionId,
        connection: BridgeConnectionStatus,
        authentication: AuthenticationPosture,
        transport_features: Vec<TransportFeatureId>,
        domain_capabilities: Vec<DomainCapabilityDescriptor>,
        domain_authorities: Vec<DomainAuthorityDescriptor>,
        diagnostics: Vec<BridgeDiagnostic>,
    ) -> Result<Self, BridgeNegotiationError> {
        if connection.state() != BridgeConnectionState::Ready {
            return Err(BridgeNegotiationError::new(
                BridgeNegotiationErrorCode::ConnectionNotReady,
                format!(
                    "negotiated receipt requires a ready connection, got {:?}",
                    connection.state()
                ),
            ));
        }

        validate_limit(
            transport_features.len(),
            MAXIMUM_TRANSPORT_FEATURES,
            "transport features",
        )?;
        validate_unique(
            &transport_features,
            BridgeNegotiationErrorCode::DuplicateTransportFeature,
            "transport feature",
        )?;
        validate_limit(
            domain_capabilities.len(),
            MAXIMUM_CAPABILITY_DOMAINS,
            "capability domains",
        )?;
        validate_unique_by(
            &domain_capabilities,
            DomainCapabilityDescriptor::domain_id,
            BridgeNegotiationErrorCode::DuplicateCapabilityDomain,
            "capability domain",
        )?;
        validate_limit(
            domain_authorities.len(),
            MAXIMUM_AUTHORITY_DOMAINS,
            "authority domains",
        )?;
        validate_unique_by(
            &domain_authorities,
            DomainAuthorityDescriptor::domain_id,
            BridgeNegotiationErrorCode::DuplicateAuthorityDomain,
            "authority domain",
        )?;
        validate_limit(diagnostics.len(), MAXIMUM_DIAGNOSTICS, "diagnostics")?;
        validate_unique_by(
            &diagnostics,
            BridgeDiagnostic::diagnostic_id,
            BridgeNegotiationErrorCode::DuplicateDiagnostic,
            "diagnostic",
        )?;

        let capability_domains: HashSet<&DomainId> = domain_capabilities
            .iter()
            .map(DomainCapabilityDescriptor::domain_id)
            .collect();
        let mut writer_scopes = HashSet::new();
        for authority in &domain_authorities {
            if !capability_domains.contains(authority.domain_id()) {
                return Err(BridgeNegotiationError::new(
                    BridgeNegotiationErrorCode::AuthorityWithoutCapability,
                    format!(
                        "authority domain {} has no capability advertisement",
                        authority.domain_id()
                    ),
                ));
            }
            if authority.write_authority() == WriteAuthority::Authoritative
                && !writer_scopes.insert(authority.scope_id())
            {
                return Err(BridgeNegotiationError::new(
                    BridgeNegotiationErrorCode::MultipleWriters,
                    format!(
                        "authority scope {} declares multiple current writers",
                        authority.scope_id()
                    ),
                ));
            }
        }

        Ok(Self {
            protocol_version: BridgeProtocolVersion::CURRENT,
            host,
            session_id,
            connection,
            authentication,
            transport_features,
            domain_capabilities,
            domain_authorities,
            diagnostics,
        })
    }

    /// Validates that every advertised domain was requested by this hello.
    pub fn validate_for(&self, request: &BridgeHelloRequest) -> Result<(), BridgeNegotiationError> {
        let requested: HashSet<&DomainId> = request.requested_domains().iter().collect();
        if let Some(domain_id) = self
            .domain_capabilities
            .iter()
            .map(DomainCapabilityDescriptor::domain_id)
            .find(|domain_id| !requested.contains(domain_id))
        {
            return Err(BridgeNegotiationError::new(
                BridgeNegotiationErrorCode::UnrequestedDomain,
                format!("receipt advertised unrequested domain {domain_id}"),
            ));
        }

        Ok(())
    }

    /// Returns the exact negotiated protocol version.
    #[must_use]
    pub const fn protocol_version(&self) -> BridgeProtocolVersion {
        self.protocol_version
    }

    /// Returns the selected host descriptor.
    #[must_use]
    pub const fn host(&self) -> &BridgeHostDescriptor {
        &self.host
    }

    /// Returns this connection session identity.
    #[must_use]
    pub const fn session_id(&self) -> &BridgeSessionId {
        &self.session_id
    }

    /// Returns the ready connection status.
    #[must_use]
    pub const fn connection(&self) -> BridgeConnectionStatus {
        self.connection
    }

    /// Returns authentication posture without inferring domain authority.
    #[must_use]
    pub const fn authentication(&self) -> AuthenticationPosture {
        self.authentication
    }

    /// Returns transport-level features.
    #[must_use]
    pub fn transport_features(&self) -> &[TransportFeatureId] {
        &self.transport_features
    }

    /// Returns domain capability advertisements.
    #[must_use]
    pub fn domain_capabilities(&self) -> &[DomainCapabilityDescriptor] {
        &self.domain_capabilities
    }

    /// Returns domain authority facts.
    #[must_use]
    pub fn domain_authorities(&self) -> &[DomainAuthorityDescriptor] {
        &self.domain_authorities
    }

    /// Returns bounded host diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[BridgeDiagnostic] {
        &self.diagnostics
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawBridgeNegotiationReceipt {
    protocol_version: BridgeProtocolVersion,
    host: BridgeHostDescriptor,
    session_id: BridgeSessionId,
    connection: BridgeConnectionStatus,
    authentication: AuthenticationPosture,
    transport_features: Vec<TransportFeatureId>,
    domain_capabilities: Vec<DomainCapabilityDescriptor>,
    domain_authorities: Vec<DomainAuthorityDescriptor>,
    diagnostics: Vec<BridgeDiagnostic>,
}

impl TryFrom<RawBridgeNegotiationReceipt> for BridgeNegotiationReceipt {
    type Error = BridgeNegotiationError;

    fn try_from(raw: RawBridgeNegotiationReceipt) -> Result<Self, Self::Error> {
        let receipt = Self::new(
            raw.host,
            raw.session_id,
            raw.connection,
            raw.authentication,
            raw.transport_features,
            raw.domain_capabilities,
            raw.domain_authorities,
            raw.diagnostics,
        )?;
        debug_assert_eq!(raw.protocol_version, receipt.protocol_version);
        Ok(receipt)
    }
}
