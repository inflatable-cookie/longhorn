use longhorn_core::HostInstanceId;
use serde::{Deserialize, Serialize};

use crate::{BridgeNegotiationError, BridgeNegotiationErrorCode};

/// Product-neutral form of the host selected for one bridge session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeHostForm {
    /// The client invokes an in-process host directly.
    Direct,
    /// The client crosses a Tauri-local command or event boundary.
    TauriLocal,
    /// The client uses a separately running local service.
    LocalService,
    /// The client uses a host on another machine or trust boundary.
    Remote,
    /// The consumer selected a local-first host with optional remote coordination.
    LocalFirst,
}

/// Stable host instance and its current topology form.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeHostDescriptor {
    /// Identity of one running host, independent of connection sessions.
    pub host_instance_id: HostInstanceId,
    /// Product-neutral form of this host.
    pub form: BridgeHostForm,
}

/// Transport and negotiation state of one bridge connection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeConnectionState {
    /// No connection attempt has started.
    Idle,
    /// A transport connection is being established.
    Connecting,
    /// Transport is available and exact-version negotiation is active.
    Negotiating,
    /// Negotiation succeeded and the bridge can carry supported work.
    Ready,
    /// The bridge remains usable with reduced transport or host posture.
    Degraded,
    /// A retryable transport connection is being re-established.
    Reconnecting,
    /// No usable transport connection exists.
    Offline,
    /// The peer protocol is incompatible.
    Incompatible,
    /// Endpoint admission or authentication rejected the connection.
    Unauthorized,
    /// A non-retryable host or transport failure occurred.
    Failed,
    /// The connection was deliberately closed.
    Closed,
}

/// Stable reason for the current bridge connection state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeConnectionReason {
    /// A consumer requested a connection.
    ConnectRequested,
    /// The transport became available.
    TransportReady,
    /// Exact-version negotiation succeeded.
    NegotiationAccepted,
    /// Advertised transport or domain capability changed.
    CapabilityChanged,
    /// The active transport was lost.
    TransportLost,
    /// Supervision scheduled a retry.
    RetryScheduled,
    /// Exact protocol versions did not match.
    VersionMismatch,
    /// Endpoint admission or authentication rejected the peer.
    AuthorizationRejected,
    /// The host or transport reported a terminal failure.
    HostFailure,
    /// The consumer deliberately shut the connection down.
    Shutdown,
}

/// Checked state and reason for one bridge connection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(try_from = "RawBridgeConnectionStatus")]
pub struct BridgeConnectionStatus {
    state: BridgeConnectionState,
    reason: Option<BridgeConnectionReason>,
}

impl BridgeConnectionStatus {
    /// Validates and constructs a connection status.
    pub fn new(
        state: BridgeConnectionState,
        reason: Option<BridgeConnectionReason>,
    ) -> Result<Self, BridgeNegotiationError> {
        let valid = matches!(
            (state, reason),
            (BridgeConnectionState::Idle, None)
                | (
                    BridgeConnectionState::Connecting,
                    Some(BridgeConnectionReason::ConnectRequested)
                )
                | (
                    BridgeConnectionState::Negotiating,
                    Some(BridgeConnectionReason::TransportReady)
                )
                | (
                    BridgeConnectionState::Ready,
                    Some(
                        BridgeConnectionReason::NegotiationAccepted
                            | BridgeConnectionReason::CapabilityChanged
                    )
                )
                | (
                    BridgeConnectionState::Degraded,
                    Some(
                        BridgeConnectionReason::CapabilityChanged
                            | BridgeConnectionReason::TransportLost
                            | BridgeConnectionReason::HostFailure
                    )
                )
                | (
                    BridgeConnectionState::Reconnecting,
                    Some(
                        BridgeConnectionReason::RetryScheduled
                            | BridgeConnectionReason::TransportLost
                    )
                )
                | (
                    BridgeConnectionState::Offline,
                    Some(BridgeConnectionReason::TransportLost)
                )
                | (
                    BridgeConnectionState::Incompatible,
                    Some(BridgeConnectionReason::VersionMismatch)
                )
                | (
                    BridgeConnectionState::Unauthorized,
                    Some(BridgeConnectionReason::AuthorizationRejected)
                )
                | (
                    BridgeConnectionState::Failed,
                    Some(BridgeConnectionReason::HostFailure)
                )
                | (
                    BridgeConnectionState::Closed,
                    Some(BridgeConnectionReason::Shutdown)
                )
        );

        if !valid {
            return Err(BridgeNegotiationError::new(
                BridgeNegotiationErrorCode::InvalidConnectionStatus,
                format!("invalid reason {reason:?} for connection state {state:?}"),
            ));
        }

        Ok(Self { state, reason })
    }

    /// Returns the current connection state.
    #[must_use]
    pub const fn state(self) -> BridgeConnectionState {
        self.state
    }

    /// Returns the reason for the current connection state.
    #[must_use]
    pub const fn reason(self) -> Option<BridgeConnectionReason> {
        self.reason
    }

    /// Returns whether supervision may retry this state.
    #[must_use]
    pub const fn reconnect_permitted(self) -> bool {
        matches!(
            self.state,
            BridgeConnectionState::Degraded
                | BridgeConnectionState::Reconnecting
                | BridgeConnectionState::Offline
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawBridgeConnectionStatus {
    state: BridgeConnectionState,
    reason: Option<BridgeConnectionReason>,
}

impl TryFrom<RawBridgeConnectionStatus> for BridgeConnectionStatus {
    type Error = BridgeNegotiationError;

    fn try_from(raw: RawBridgeConnectionStatus) -> Result<Self, Self::Error> {
        Self::new(raw.state, raw.reason)
    }
}

/// Authentication fact reported by the selected host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationPosture {
    /// This host does not require authentication for the negotiated session.
    NotRequired,
    /// The host requires authentication before protected domains are available.
    Required,
    /// The host accepted authentication for this session.
    Authenticated,
    /// The host rejected authentication for this session.
    Rejected,
}
