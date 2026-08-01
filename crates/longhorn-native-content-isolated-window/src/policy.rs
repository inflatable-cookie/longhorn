use std::time::Duration;

use longhorn_core::WindowId;
use longhorn_native_content::{
    DetachPolicy, InputRoutingMode, MechanismCapabilities, NativeContentIslandId,
    NativeContentMechanism,
};

/// Honest capabilities of the isolated-window coordination layer.
pub const ISOLATED_WINDOW_CAPABILITIES: MechanismCapabilities = MechanismCapabilities::new(
    NativeContentMechanism::IsolatedWindow,
    InputRoutingMode::NativeDirect,
    true,
    DetachPolicy::OwnerProcessTermination,
    true,
    true,
);

/// Immutable island mapping and bounded owner-process policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsolatedWindowSpec {
    island_id: NativeContentIslandId,
    host_window_id: WindowId,
    request_timeout: Duration,
    teardown_timeout: Duration,
}

impl IsolatedWindowSpec {
    /// Creates one mapping without product content or outer placement policy.
    #[must_use]
    pub const fn new(
        island_id: NativeContentIslandId,
        host_window_id: WindowId,
        request_timeout: Duration,
        teardown_timeout: Duration,
    ) -> Self {
        Self {
            island_id,
            host_window_id,
            request_timeout,
            teardown_timeout,
        }
    }

    /// Returns shared island identity.
    #[must_use]
    pub const fn island_id(&self) -> &NativeContentIslandId {
        &self.island_id
    }

    /// Returns the logical outer-window binding without granting placement authority.
    #[must_use]
    pub const fn host_window_id(&self) -> &WindowId {
        &self.host_window_id
    }

    /// Returns the maximum wait for one correlated helper request.
    #[must_use]
    pub const fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    /// Returns the maximum wait for owner-process teardown.
    #[must_use]
    pub const fn teardown_timeout(&self) -> Duration {
        self.teardown_timeout
    }
}
