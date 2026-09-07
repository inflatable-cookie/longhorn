//! Registration identities and positive dispatch limits.

use crate::{DispatchError, DispatchErrorKind, types::validate_identity};
use std::time::Duration;

/// Exact schema version and digest supplied by the registration owner.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SchemaIdentity {
    version: String,
    digest: String,
}

impl SchemaIdentity {
    /// Creates one bounded schema identity without carrying a schema body.
    pub fn new(
        version: impl Into<String>,
        digest: impl Into<String>,
    ) -> Result<Self, DispatchError> {
        let version = version.into();
        let digest = digest.into();
        validate_identity(&version)?;
        validate_identity(&digest)?;
        Ok(Self { version, digest })
    }

    /// Returns the exact schema version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the exact schema digest.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

/// Executor class bound to one registered capability.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExecutionKind {
    /// Provider-native callback dispatched to the linked host.
    NativeClient,
    /// Host-mediated MCP callback.
    Mcp,
    /// Exact application callback without MCP identity.
    App,
}

/// Consumer-declared effect posture used to prohibit replay.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DispatchEffect {
    /// The callback has no external effect.
    ReadOnly,
    /// The callback may mutate consumer state.
    Mutating,
    /// The consumer cannot prove the callback's effect posture.
    Indeterminate,
}

/// Positive limits fixed before callback dispatch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DispatchBounds {
    max_outstanding_calls: usize,
    max_argument_bytes: usize,
    max_result_bytes: usize,
    max_progress_item_bytes: usize,
    max_queued_progress_items: usize,
    max_call_duration: Duration,
}

impl DispatchBounds {
    /// Creates one positive bound set.
    pub fn new(
        max_outstanding_calls: usize,
        max_argument_bytes: usize,
        max_result_bytes: usize,
        max_progress_item_bytes: usize,
        max_queued_progress_items: usize,
        max_call_duration: Duration,
    ) -> Result<Self, DispatchError> {
        let bounds = Self {
            max_outstanding_calls,
            max_argument_bytes,
            max_result_bytes,
            max_progress_item_bytes,
            max_queued_progress_items,
            max_call_duration,
        };
        bounds.validate()?;
        Ok(bounds)
    }

    pub(crate) fn validate(self) -> Result<(), DispatchError> {
        if self.max_outstanding_calls == 0
            || self.max_argument_bytes == 0
            || self.max_result_bytes == 0
            || self.max_progress_item_bytes == 0
            || self.max_queued_progress_items == 0
            || self.max_call_duration.is_zero()
        {
            Err(DispatchError::new(DispatchErrorKind::InvalidBounds))
        } else {
            Ok(())
        }
    }

    pub(crate) fn fits_within(self, ceiling: Self) -> bool {
        self.max_outstanding_calls <= ceiling.max_outstanding_calls
            && self.max_argument_bytes <= ceiling.max_argument_bytes
            && self.max_result_bytes <= ceiling.max_result_bytes
            && self.max_progress_item_bytes <= ceiling.max_progress_item_bytes
            && self.max_queued_progress_items <= ceiling.max_queued_progress_items
            && self.max_call_duration <= ceiling.max_call_duration
    }

    /// Returns the maximum concurrent callbacks.
    #[must_use]
    pub const fn max_outstanding_calls(self) -> usize {
        self.max_outstanding_calls
    }

    /// Returns the maximum argument bytes.
    #[must_use]
    pub const fn max_argument_bytes(self) -> usize {
        self.max_argument_bytes
    }

    /// Returns the maximum result bytes.
    #[must_use]
    pub const fn max_result_bytes(self) -> usize {
        self.max_result_bytes
    }

    /// Returns the maximum progress-item bytes.
    #[must_use]
    pub const fn max_progress_item_bytes(self) -> usize {
        self.max_progress_item_bytes
    }

    /// Returns the maximum queued progress items.
    #[must_use]
    pub const fn max_queued_progress_items(self) -> usize {
        self.max_queued_progress_items
    }

    /// Returns the maximum call duration.
    #[must_use]
    pub const fn max_call_duration(self) -> Duration {
        self.max_call_duration
    }
}

/// Exact capability declaration normalized by the composing host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredCapability {
    registration_revision: String,
    capability: String,
    input_schema: SchemaIdentity,
    output_schema: SchemaIdentity,
    execution_kind: ExecutionKind,
    effect: DispatchEffect,
    bounds: DispatchBounds,
}

impl RegisteredCapability {
    /// Creates one exact registration without carrying schema bodies.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registration_revision: impl Into<String>,
        capability: impl Into<String>,
        input_schema: SchemaIdentity,
        output_schema: SchemaIdentity,
        execution_kind: ExecutionKind,
        effect: DispatchEffect,
        bounds: DispatchBounds,
    ) -> Result<Self, DispatchError> {
        let registration_revision = registration_revision.into();
        let capability = capability.into();
        validate_identity(&registration_revision)?;
        validate_identity(&capability)?;
        bounds.validate()?;
        Ok(Self {
            registration_revision,
            capability,
            input_schema,
            output_schema,
            execution_kind,
            effect,
            bounds,
        })
    }

    /// Returns the exact registration revision.
    #[must_use]
    pub fn registration_revision(&self) -> &str {
        &self.registration_revision
    }

    /// Returns the selected capability identity.
    #[must_use]
    pub fn capability(&self) -> &str {
        &self.capability
    }

    /// Returns the exact input schema identity.
    #[must_use]
    pub const fn input_schema(&self) -> &SchemaIdentity {
        &self.input_schema
    }

    /// Returns the exact output schema identity.
    #[must_use]
    pub const fn output_schema(&self) -> &SchemaIdentity {
        &self.output_schema
    }

    /// Returns the bound execution kind.
    #[must_use]
    pub const fn execution_kind(&self) -> ExecutionKind {
        self.execution_kind
    }

    /// Returns the declared effect posture.
    #[must_use]
    pub const fn effect(&self) -> DispatchEffect {
        self.effect
    }

    /// Returns the declared positive bounds.
    #[must_use]
    pub const fn bounds(&self) -> DispatchBounds {
        self.bounds
    }
}
