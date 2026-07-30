use longhorn_core::{BridgeCapabilityId, BridgeDiagnosticId, DomainId};
use serde::{Deserialize, Serialize};

use crate::{BridgeNegotiationError, BridgeNegotiationErrorCode};

use super::validation::{validate_limit, validate_unique};

/// Maximum capabilities one domain descriptor may advertise.
pub const MAXIMUM_CAPABILITIES_PER_DOMAIN: usize = 128;
/// Maximum UTF-8 bytes in one diagnostic message.
pub const MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES: usize = 4_096;

/// Checked capability advertisement for one domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    try_from = "RawDomainCapabilityDescriptor"
)]
pub struct DomainCapabilityDescriptor {
    domain_id: DomainId,
    capabilities: Vec<BridgeCapabilityId>,
}

impl DomainCapabilityDescriptor {
    /// Constructs a bounded, nonempty capability advertisement.
    pub fn new(
        domain_id: DomainId,
        capabilities: Vec<BridgeCapabilityId>,
    ) -> Result<Self, BridgeNegotiationError> {
        if capabilities.is_empty() {
            return Err(BridgeNegotiationError::new(
                BridgeNegotiationErrorCode::EmptyCapabilityDomain,
                format!("domain {domain_id} must advertise at least one capability"),
            ));
        }
        validate_limit(
            capabilities.len(),
            MAXIMUM_CAPABILITIES_PER_DOMAIN,
            "capabilities per domain",
        )?;
        validate_unique(
            &capabilities,
            BridgeNegotiationErrorCode::DuplicateCapability,
            "capability",
        )?;

        Ok(Self {
            domain_id,
            capabilities,
        })
    }

    /// Returns the advertised domain.
    #[must_use]
    pub const fn domain_id(&self) -> &DomainId {
        &self.domain_id
    }

    /// Returns capabilities without granting authority.
    #[must_use]
    pub fn capabilities(&self) -> &[BridgeCapabilityId] {
        &self.capabilities
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawDomainCapabilityDescriptor {
    domain_id: DomainId,
    capabilities: Vec<BridgeCapabilityId>,
}

impl TryFrom<RawDomainCapabilityDescriptor> for DomainCapabilityDescriptor {
    type Error = BridgeNegotiationError;

    fn try_from(raw: RawDomainCapabilityDescriptor) -> Result<Self, Self::Error> {
        Self::new(raw.domain_id, raw.capabilities)
    }
}

/// Checked host diagnostic attached to a negotiated receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    try_from = "RawBridgeDiagnostic"
)]
pub struct BridgeDiagnostic {
    diagnostic_id: BridgeDiagnosticId,
    message: String,
}

impl BridgeDiagnostic {
    /// Constructs a bounded diagnostic.
    pub fn new(
        diagnostic_id: BridgeDiagnosticId,
        message: impl Into<String>,
    ) -> Result<Self, BridgeNegotiationError> {
        let message = message.into();
        if message.is_empty() {
            return Err(BridgeNegotiationError::new(
                BridgeNegotiationErrorCode::EmptyDiagnostic,
                format!("diagnostic {diagnostic_id} has an empty message"),
            ));
        }
        validate_limit(
            message.len(),
            MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES,
            "diagnostic message bytes",
        )?;

        Ok(Self {
            diagnostic_id,
            message,
        })
    }

    /// Returns the stable diagnostic category.
    #[must_use]
    pub const fn diagnostic_id(&self) -> &BridgeDiagnosticId {
        &self.diagnostic_id
    }

    /// Returns the bounded human-readable diagnostic.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawBridgeDiagnostic {
    diagnostic_id: BridgeDiagnosticId,
    message: String,
}

impl TryFrom<RawBridgeDiagnostic> for BridgeDiagnostic {
    type Error = BridgeNegotiationError;

    fn try_from(raw: RawBridgeDiagnostic) -> Result<Self, Self::Error> {
        Self::new(raw.diagnostic_id, raw.message)
    }
}
