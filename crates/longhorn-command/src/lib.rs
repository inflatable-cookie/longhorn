//! Pure bounded command registry, context, argument, availability, admission,
//! keyboard, keymap, execution-port, and discovery primitives.

mod argument;
mod availability;
mod context;
mod declaration;
mod error;
mod execution;
mod identity;
mod keyboard;
mod keymap;
mod limits;
mod registry;
mod search;

pub use argument::{
    CommandArgumentField, CommandArgumentKind, CommandArgumentSchema, CommandArgumentValue,
    CommandArguments, CommandFiniteNumber,
};
pub use availability::{
    CommandAvailability, CommandAvailabilityReason, CommandAvailabilityReasonCode,
    CommandAvailabilityRecord, CommandAvailabilitySnapshot, CommandAvailabilityState,
    CommandDiagnostic, CommandDiagnosticError, CommandEvidence, MAXIMUM_COMMAND_DIAGNOSTIC_BYTES,
};
pub use context::{
    CommandCapabilitySnapshot, CommandCapabilitySnapshotError, CommandContextRevision,
    CommandContextSnapshot, CommandContextSnapshotError,
};
pub use declaration::{
    CommandCapabilityDefinition, CommandContextDefinition, CommandDefinition, CommandKeyword,
    CommandSurface, CommandTextInputPolicy, CommandVisibility,
};
pub use error::{
    CommandArgumentError, CommandArgumentErrorCode, CommandRegistryError, CommandRegistryErrorCode,
    CommandSearchError,
};
pub use execution::{
    AdmittedCommandInvocation, CommandAdmissionEngine, CommandAvailabilityProjectionError,
    CommandAvailabilitySource, CommandCapabilitySource, CommandContextSource,
    CommandExecutionOutcome, CommandExecutionRequest, CommandExecutionResult, CommandExecutor,
    CommandExecutorOutcome, CommandFailure, CommandFailureCode, CommandFailurePhase,
    CommandSourceFailure,
};
pub use identity::{CommandRegistryDigest, CommandRegistryGeneration};
pub use keyboard::{
    CommandKeyChord, CommandKeyTrigger, CommandKeyboardInput, CommandKeyboardMode,
    CommandModifierError, CommandModifiers, CommandNativeModifier, CommandPhysicalCode,
    CommandPhysicalCodeError, CommandPlatform, CommandPlatformScope, CommandReservedChordPolicy,
    CommandTriggerModifiers, NoReservedCommandChords,
};
pub use keymap::{
    CommandBindingCandidate, CommandBindingDefinition, CommandBindingReplacement,
    CommandBindingSource, CommandBindingWinner, CommandCandidateDisposition,
    CommandEffectiveBinding, CommandEffectiveKeymap, CommandInvocation, CommandKeyResolution,
    CommandKeyResolutionError, CommandKeyboardGate, CommandKeymapConflict, CommandKeymapError,
    CommandKeymapErrorCode, CommandKeymapOverride, CommandKeymapPreset, CommandShortcutRecord,
};
pub use limits::CommandLimits;
pub use registry::{CommandDiscoveryRecord, CommandRegistry, CommandRegistryBuilder};
pub use search::CommandSearchHit;
