use longhorn_settings::{SettingsApplyCommand, SettingsMutationResult, SettingsResetCommand};

/// Explicit non-config authority for a broader failure-atomic settings unit.
///
/// Implementing this trait is a consumer claim. The built-in config adapter
/// neither implements nor composes it from sequential domain writes.
pub trait ConsumerSettingsTransactionAuthority {
    /// Consumer-owned operational failure.
    type Error;
    /// Separate evidence from the authority that promises atomicity.
    type Receipt;

    /// Executes one broader apply transaction.
    fn apply_transaction(
        &self,
        command: &SettingsApplyCommand,
    ) -> Result<ConsumerSettingsTransactionOutcome<Self::Receipt>, Self::Error>;

    /// Executes one broader reset transaction.
    fn reset_transaction(
        &self,
        command: &SettingsResetCommand,
    ) -> Result<ConsumerSettingsTransactionOutcome<Self::Receipt>, Self::Error>;
}

/// Settings result paired with separate consumer transaction evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumerSettingsTransactionOutcome<R> {
    /// Ordinary settings protocol result.
    pub result: SettingsMutationResult,
    /// Consumer authority evidence; never a config publication receipt.
    pub authority_receipt: R,
}
