use std::{error::Error, fmt};

/// Stable category for bridge negotiation or authority validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeNegotiationErrorCode {
    /// The peer uses a protocol version this crate does not accept.
    IncompatibleProtocol,
    /// A bounded descriptor collection exceeded its limit.
    LimitExceeded,
    /// A requested domain was repeated.
    DuplicateRequestedDomain,
    /// A transport feature was repeated.
    DuplicateTransportFeature,
    /// A domain capability descriptor was repeated.
    DuplicateCapabilityDomain,
    /// One capability was repeated within a domain descriptor.
    DuplicateCapability,
    /// A domain capability descriptor advertised no capability.
    EmptyCapabilityDomain,
    /// A domain authority descriptor was repeated.
    DuplicateAuthorityDomain,
    /// An authority descriptor names a domain with no capability descriptor.
    AuthorityWithoutCapability,
    /// A receipt names a domain absent from the hello request.
    UnrequestedDomain,
    /// More than one domain descriptor claims current write authority for a scope.
    MultipleWriters,
    /// A connection state and reason combination is invalid.
    InvalidConnectionStatus,
    /// A negotiated receipt does not describe a ready connection.
    ConnectionNotReady,
    /// An authority epoch was zero.
    InvalidAuthorityEpoch,
    /// An authority descriptor combines incompatible availability or ownership facts.
    InvalidAuthorityDescriptor,
    /// A diagnostic identifier was repeated.
    DuplicateDiagnostic,
    /// A diagnostic message was empty.
    EmptyDiagnostic,
}

/// Checked bridge negotiation or authority validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeNegotiationError {
    code: BridgeNegotiationErrorCode,
    detail: String,
}

impl BridgeNegotiationError {
    pub(crate) fn new(code: BridgeNegotiationErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    /// Returns the stable failure category.
    #[must_use]
    pub const fn code(&self) -> BridgeNegotiationErrorCode {
        self.code
    }

    /// Returns bounded-context diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for BridgeNegotiationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for BridgeNegotiationError {}
