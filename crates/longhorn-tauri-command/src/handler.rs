use std::sync::Mutex;

use longhorn_command_config::{
    CommandCatalogueSnapshot, CommandKeymapCommit, CommandKeymapLoadOutcome,
    CommandKeymapMutationResult, CommandKeymapPreview, CommandKeymapPreviewResult,
    CommandKeymapReset,
};

use crate::{CommandHostAuthority, CommandHostError, CommandHostService};

/// Shared command assembly used by Tauri and direct/serialized tests.
pub struct CommandHandlerAssembly<A> {
    authority: Mutex<A>,
}

impl<A> CommandHandlerAssembly<A> {
    /// Binds one explicitly injected authority.
    #[must_use]
    pub const fn new(authority: A) -> Self {
        Self {
            authority: Mutex::new(authority),
        }
    }

    /// Runs trusted host work against the injected authority.
    pub fn with_authority<Output>(
        &self,
        action: impl FnOnce(&mut A) -> Output,
    ) -> Result<Output, CommandHostError> {
        self.authority
            .lock()
            .map(|mut authority| action(&mut authority))
            .map_err(|_| CommandHostError::state_unavailable())
    }
}

impl<A> CommandHostService for CommandHandlerAssembly<A>
where
    A: CommandHostAuthority,
{
    fn catalogue(&self, caller: &str) -> Result<CommandCatalogueSnapshot, CommandHostError> {
        self.with_authority(|authority| authority.catalogue(caller))?
    }

    fn keymap(&self, caller: &str) -> Result<CommandKeymapLoadOutcome, CommandHostError> {
        self.with_authority(|authority| authority.keymap(caller))?
    }

    fn preview(
        &self,
        caller: &str,
        request: CommandKeymapPreview,
    ) -> Result<CommandKeymapPreviewResult, CommandHostError> {
        self.with_authority(|authority| authority.preview(caller, request))?
    }

    fn commit(
        &self,
        caller: &str,
        request: CommandKeymapCommit,
    ) -> Result<CommandKeymapMutationResult, CommandHostError> {
        self.with_authority(|authority| authority.commit(caller, request))?
    }

    fn reset(
        &self,
        caller: &str,
        request: CommandKeymapReset,
    ) -> Result<CommandKeymapMutationResult, CommandHostError> {
        self.with_authority(|authority| authority.reset(caller, request))?
    }
}
