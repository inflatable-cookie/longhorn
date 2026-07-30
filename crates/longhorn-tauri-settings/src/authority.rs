use longhorn_settings::{
    SettingsApplyCommand, SettingsLoadCommand, SettingsLoadOutcome, SettingsMutationResult,
    SettingsRegistrySnapshot, SettingsResetCommand,
};

use crate::SettingsHostError;

/// Consumer-injected registry, authorization, and mutation authority.
pub trait SettingsAuthority: Send {
    /// Returns the caller-authorized sealed registry projection.
    fn registry(&mut self, caller: &str) -> Result<SettingsRegistrySnapshot, SettingsHostError>;

    /// Loads one caller-authorized scope.
    fn load(
        &mut self,
        caller: &str,
        command: SettingsLoadCommand,
    ) -> Result<SettingsLoadOutcome, SettingsHostError>;

    /// Applies one caller-authorized product intent.
    fn apply(
        &mut self,
        caller: &str,
        command: SettingsApplyCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError>;

    /// Resets caller-authorized user overrides.
    fn reset(
        &mut self,
        caller: &str,
        command: SettingsResetCommand,
    ) -> Result<SettingsMutationResult, SettingsHostError>;
}
