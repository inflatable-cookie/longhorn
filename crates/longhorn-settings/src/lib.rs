//! Pure settings composition and authority protocol.
//!
//! Consumers own product schemas and mutation semantics. This crate validates
//! product-neutral declarations, seals deterministic registries, and carries
//! opaque values between an authoritative host and renderer clients.

mod declaration;
mod error;
mod limits;
mod protocol;
mod registry;
mod value;

pub use declaration::{
    SettingsAnchorDefinition, SettingsApplyUnitDefinition, SettingsCapabilityDefinition,
    SettingsModuleDefinition, SettingsMutationTiming, SettingsPageDefinition, SettingsPageFeatures,
    SettingsRendererDefinition, SettingsScopeDefinition, SettingsSectionDefinition,
};
pub use error::{SettingsRegistryError, SettingsRegistryErrorCode};
pub use limits::SettingsLimits;
pub use protocol::{
    SETTINGS_PROTOCOL_VERSION, SettingsActivationRequirement, SettingsActivationState,
    SettingsApplyCommand, SettingsAuthorityExpectation, SettingsConflict,
    SettingsDurabilityEvidence, SettingsEditability, SettingsEffectiveSource, SettingsLoadCommand,
    SettingsLoadOutcome, SettingsMutationOutcome, SettingsMutationReceipt, SettingsMutationResult,
    SettingsPolicyEffect, SettingsPolicyProjection, SettingsProtocolError, SettingsProtocolVersion,
    SettingsRecoveryCode, SettingsRecoveryState, SettingsRegistryChangedEvent,
    SettingsRegistrySnapshot, SettingsRejection, SettingsRejectionCode, SettingsResetCommand,
    SettingsScopeChangedEvent, SettingsScopeRevision, SettingsScopeSnapshot,
    SettingsSourceDiagnostic, SettingsValueProjection,
};
pub use registry::{
    SettingsRegistry, SettingsRegistryBuilder, SettingsRegistryDigest, SettingsRegistryGeneration,
};
pub use value::{SettingsOpaqueValue, SettingsOpaqueValueError};
