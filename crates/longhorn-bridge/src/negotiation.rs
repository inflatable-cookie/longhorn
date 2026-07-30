mod advertisement;
mod hello;
mod receipt;
mod validation;

pub use advertisement::{
    BridgeDiagnostic, DomainCapabilityDescriptor, MAXIMUM_CAPABILITIES_PER_DOMAIN,
    MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES,
};
pub use hello::{BridgeHelloRequest, MAXIMUM_REQUESTED_DOMAINS};
pub use receipt::{
    BridgeNegotiationReceipt, MAXIMUM_AUTHORITY_DOMAINS, MAXIMUM_CAPABILITY_DOMAINS,
    MAXIMUM_DIAGNOSTICS, MAXIMUM_TRANSPORT_FEATURES,
};
