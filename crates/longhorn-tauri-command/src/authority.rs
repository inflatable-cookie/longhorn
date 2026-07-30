use longhorn_command_config::{
    CommandCatalogueSnapshot, CommandKeymapCommit, CommandKeymapLoadOutcome,
    CommandKeymapMutationResult, CommandKeymapPreview, CommandKeymapPreviewResult,
    CommandKeymapReset,
};

use crate::CommandHostError;

/// Consumer-injected catalogue, authorization, and keymap authority.
pub trait CommandHostAuthority: Send {
    /// Returns the caller-authorized sealed catalogue.
    fn catalogue(&mut self, caller: &str) -> Result<CommandCatalogueSnapshot, CommandHostError>;

    /// Loads the caller-authorized effective keymap.
    fn keymap(&mut self, caller: &str) -> Result<CommandKeymapLoadOutcome, CommandHostError>;

    /// Previews one caller-authorized keymap patch.
    fn preview(
        &mut self,
        caller: &str,
        request: CommandKeymapPreview,
    ) -> Result<CommandKeymapPreviewResult, CommandHostError>;

    /// Commits one exact caller-authorized preview.
    fn commit(
        &mut self,
        caller: &str,
        request: CommandKeymapCommit,
    ) -> Result<CommandKeymapMutationResult, CommandHostError>;

    /// Resets caller-authorized keymap state.
    fn reset(
        &mut self,
        caller: &str,
        request: CommandKeymapReset,
    ) -> Result<CommandKeymapMutationResult, CommandHostError>;
}
