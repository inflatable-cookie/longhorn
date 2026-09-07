//! Redacted failure vocabulary for the dispatch boundary.

use std::fmt;

/// Machine-distinct class of a dispatch refusal or terminal failure.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DispatchErrorKind {
    /// A required identity is blank, oversized, or contains control bytes.
    InvalidIdentity,
    /// At least one declared numeric or duration bound is not positive.
    InvalidBounds,
    /// The call names a registration revision other than the configured one.
    StaleRegistration,
    /// The call names another registered capability.
    UnknownCapability,
    /// A schema version differs from the configured exact version.
    UnsupportedSchemaVersion,
    /// A schema digest differs from the configured exact digest.
    UnknownSchema,
    /// The execution kind differs from the configured kind.
    ExecutionKindMismatch,
    /// The effect posture differs from the configured posture.
    EffectMismatch,
    /// Stable admission identity belongs to another consumer attempt.
    ForeignBinding,
    /// A generation belongs to an earlier or otherwise different attempt.
    StaleBinding,
    /// The consumer reports that the fixed admission binding is revoked.
    RevokedBinding,
    /// A payload or concurrent-call bound was exceeded.
    LimitExceeded,
    /// The trusted host reports an elapsed or zero remaining deadline.
    DeadlineExceeded,
    /// The call identity was already admitted.
    DuplicateCall,
    /// A second result was offered for the same completed call.
    DuplicateResult,
    /// A result arrived after cancellation or another terminal failure.
    PostTerminalResult,
    /// The in-flight call was cancelled.
    Cancelled,
    /// The callback returned without recording one result.
    MissingResult,
    /// The consumer callback reported a typed execution failure.
    CallbackFailed,
}

impl DispatchErrorKind {
    /// Returns the stable diagnostic code for this class.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidIdentity => "longhorn.agent_tool_dispatch.invalid_identity",
            Self::InvalidBounds => "longhorn.agent_tool_dispatch.invalid_bounds",
            Self::StaleRegistration => "longhorn.agent_tool_dispatch.stale_registration",
            Self::UnknownCapability => "longhorn.agent_tool_dispatch.unknown_capability",
            Self::UnsupportedSchemaVersion => {
                "longhorn.agent_tool_dispatch.unsupported_schema_version"
            }
            Self::UnknownSchema => "longhorn.agent_tool_dispatch.unknown_schema",
            Self::ExecutionKindMismatch => "longhorn.agent_tool_dispatch.execution_kind_mismatch",
            Self::EffectMismatch => "longhorn.agent_tool_dispatch.effect_mismatch",
            Self::ForeignBinding => "longhorn.agent_tool_dispatch.foreign_binding",
            Self::StaleBinding => "longhorn.agent_tool_dispatch.stale_binding",
            Self::RevokedBinding => "longhorn.agent_tool_dispatch.revoked_binding",
            Self::LimitExceeded => "longhorn.agent_tool_dispatch.limit_exceeded",
            Self::DeadlineExceeded => "longhorn.agent_tool_dispatch.deadline_exceeded",
            Self::DuplicateCall => "longhorn.agent_tool_dispatch.duplicate_call",
            Self::DuplicateResult => "longhorn.agent_tool_dispatch.duplicate_result",
            Self::PostTerminalResult => "longhorn.agent_tool_dispatch.post_terminal_result",
            Self::Cancelled => "longhorn.agent_tool_dispatch.cancelled",
            Self::MissingResult => "longhorn.agent_tool_dispatch.missing_result",
            Self::CallbackFailed => "longhorn.agent_tool_dispatch.callback_failed",
        }
    }

    /// Returns the fixed redacted message for this class.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidIdentity => "A required dispatch identity was rejected",
            Self::InvalidBounds => "Dispatch bounds must be positive",
            Self::StaleRegistration => "The registration revision is stale",
            Self::UnknownCapability => "The registered capability is unknown",
            Self::UnsupportedSchemaVersion => "The schema version is unsupported",
            Self::UnknownSchema => "The schema digest is unknown",
            Self::ExecutionKindMismatch => "The execution kind does not match",
            Self::EffectMismatch => "The effect posture does not match",
            Self::ForeignBinding => "The trusted binding belongs to another attempt",
            Self::StaleBinding => "The trusted binding generation is stale",
            Self::RevokedBinding => "The trusted binding is revoked",
            Self::LimitExceeded => "A dispatch bound was exceeded",
            Self::DeadlineExceeded => "The dispatch deadline elapsed",
            Self::DuplicateCall => "The call was already admitted",
            Self::DuplicateResult => "A result was already accepted",
            Self::PostTerminalResult => "A result arrived after terminal settlement",
            Self::Cancelled => "The in-flight call was cancelled",
            Self::MissingResult => "The callback returned without one result",
            Self::CallbackFailed => "The consumer callback failed",
        }
    }
}

/// Redacted diagnostic suitable for logs and fixture receipts.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SafeDiagnostic {
    code: &'static str,
    message: &'static str,
}

impl SafeDiagnostic {
    pub(crate) const fn for_kind(kind: DispatchErrorKind) -> Self {
        Self {
            code: kind.code(),
            message: kind.message(),
        }
    }

    /// Returns the stable machine code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.code
    }

    /// Returns the fixed redacted message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        self.message
    }
}

/// Typed dispatch refusal or terminal failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DispatchError {
    kind: DispatchErrorKind,
}

impl DispatchError {
    /// Creates one redacted error of an exact class.
    #[must_use]
    pub const fn new(kind: DispatchErrorKind) -> Self {
        Self { kind }
    }

    /// Returns the exact failure class.
    #[must_use]
    pub const fn kind(self) -> DispatchErrorKind {
        self.kind
    }

    /// Returns the safe diagnostic for this failure.
    #[must_use]
    pub const fn diagnostic(self) -> SafeDiagnostic {
        SafeDiagnostic::for_kind(self.kind)
    }
}

impl fmt::Display for DispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.message())
    }
}

impl std::error::Error for DispatchError {}

/// Typed, detail-free failure returned by a consumer callback.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CallbackFailure;

impl CallbackFailure {
    /// Creates one redacted callback failure.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
