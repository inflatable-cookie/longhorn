use longhorn_core::PhysicalSize;
use longhorn_native_content::{AttachGeneration, NativeContentFailureCode, NativeContentRequestId};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Current exact isolated-window helper protocol version.
pub const ISOLATED_WINDOW_HELPER_PROTOCOL_VERSION: u32 = 1;

/// Exact isolated-window helper protocol version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IsolatedWindowHelperProtocolVersion(u32);

impl IsolatedWindowHelperProtocolVersion {
    /// Current supported helper protocol version.
    pub const CURRENT: Self = Self(ISOLATED_WINDOW_HELPER_PROTOCOL_VERSION);

    /// Returns the serialized version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Serialize for IsolatedWindowHelperProtocolVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for IsolatedWindowHelperProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version = u32::deserialize(deserializer)?;
        if version == ISOLATED_WINDOW_HELPER_PROTOCOL_VERSION {
            Ok(Self(version))
        } else {
            Err(de::Error::custom(format_args!(
                "isolated-window helper protocol {version} is unsupported; expected {ISOLATED_WINDOW_HELPER_PROTOCOL_VERSION}"
            )))
        }
    }
}

/// One generation- and correlation-bound command sent to a content owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HelperCommand {
    /// Exact helper protocol line.
    pub protocol_version: IsolatedWindowHelperProtocolVersion,
    /// Attach generation that owns the command.
    pub generation: AttachGeneration,
    /// Bounded request correlation identity.
    pub request_id: NativeContentRequestId,
    /// Product-free helper operation.
    pub command: HelperCommandKind,
}

/// Closed product-free command vocabulary for an isolated content owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HelperCommandKind {
    /// Apply physical content-area size. Outer position is intentionally absent.
    SetContentSize {
        /// Requested physical content-area size.
        size: PhysicalSize,
    },
    /// Show isolated native content.
    Show,
    /// Hide isolated native content.
    Hide,
    /// Request native focus.
    Focus,
    /// Release native focus when owned.
    ReleaseFocus,
    /// Apply an admitted native resize capability hint.
    SetResizable {
        /// Whether native resize is enabled.
        resizable: bool,
    },
    /// Return fresh native state.
    Observe,
    /// Cooperatively close content and exit.
    Shutdown,
}

/// One generation-bound message emitted by a content owner.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HelperMessage {
    /// Exact helper protocol line.
    pub protocol_version: IsolatedWindowHelperProtocolVersion,
    /// Attach generation that owns the message.
    pub generation: AttachGeneration,
    /// Product-free helper evidence.
    pub message: HelperMessageKind,
}

/// Closed product-free helper evidence vocabulary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum HelperMessageKind {
    /// Bounded startup progress before readiness.
    Progress {
        /// Consumer-defined bounded startup phase.
        phase: String,
    },
    /// Native content is attached and ready under consumer-defined readiness.
    Ready {
        /// Fresh state at readiness.
        snapshot: HelperSnapshot,
        /// Operating-system disposable owner process id.
        owner_process_id: u32,
        /// Confirms consumer-owned native content was attached.
        native_content_attached: bool,
    },
    /// One command completed or failed.
    Acknowledged {
        /// Correlated command identity.
        request_id: NativeContentRequestId,
        /// Whether the command completed successfully.
        applied: bool,
        /// Stable failure evidence when rejected.
        failure: Option<NativeContentFailureCode>,
        /// Fresh state after the command when available.
        snapshot: Option<HelperSnapshot>,
    },
    /// Consumer-owned content requested policy admission.
    ContentRequest {
        /// Correlated request awaiting consumer policy.
        request: IsolatedContentRequest,
    },
    /// Effective native focus changed.
    FocusChanged {
        /// Whether native content is focused.
        focused: bool,
    },
    /// Effective native visibility changed.
    VisibilityChanged {
        /// Whether native content is visible.
        visible: bool,
    },
    /// Cooperative owner teardown completed.
    TeardownCompleted,
}

/// Fresh observable isolated content state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HelperSnapshot {
    /// Physical content-area size, excluding outer placement.
    pub content_size: PhysicalSize,
    /// Effective native visibility.
    pub visible: bool,
    /// Effective native focus.
    pub focused: bool,
}

/// One bounded, generation-protected request from consumer-owned content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IsolatedContentRequest {
    /// Correlates the consumer decision or later desired update.
    pub request_id: NativeContentRequestId,
    /// Product-free request category.
    pub request: IsolatedContentRequestKind,
}

/// Requests that isolated native content may submit for consumer policy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum IsolatedContentRequestKind {
    /// Propose content-area size without changing durable desired state.
    Resize {
        /// Requested physical content-area size.
        size: PhysicalSize,
    },
    /// Ask consumer policy to show the content.
    Show,
    /// Ask consumer policy to hide the content.
    Hide,
    /// Ask consumer policy to close the current generation.
    Close,
    /// Report whether native host resize should be enabled.
    ResizeHint {
        /// Whether native host resize should be enabled.
        resizable: bool,
    },
}
