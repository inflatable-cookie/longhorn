use longhorn_bridge::{BridgeHelloRequest, BridgeNegotiationReceipt, DomainCapabilityDescriptor};

use crate::BridgeHostError;

/// Consumer-injected negotiation and current authority provider.
pub trait BridgeAuthorityProvider: Send {
    /// Negotiates one caller session against the registered domain set.
    fn negotiate(
        &mut self,
        caller: &str,
        request: &BridgeHelloRequest,
        registered_domains: &[DomainCapabilityDescriptor],
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError>;

    /// Refreshes capability and authority facts for an existing session.
    fn refresh(
        &mut self,
        caller: &str,
        current: &BridgeNegotiationReceipt,
    ) -> Result<BridgeNegotiationReceipt, BridgeHostError>;
}
