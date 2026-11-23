use std::sync::Mutex;

use longhorn_command_config::{
    CommandCatalogueSnapshot, CommandKeymapCommit, CommandKeymapLoadOutcome,
    CommandKeymapMutationResult, CommandKeymapPreview, CommandKeymapPreviewResult,
    CommandKeymapReset,
};

use crate::{CommandHostAuthority, CommandHostError};

/// Shared command assembly used by Tauri and direct/serialized tests.
pub struct CommandHandlerAssembly<A: ?Sized> {
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
}

impl<A: ?Sized> CommandHandlerAssembly<A> {
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

impl<A> CommandHandlerAssembly<A>
where
    A: CommandHostAuthority + ?Sized,
{
    /// Returns one caller-authorized sealed catalogue.
    pub fn catalogue(&self, caller: &str) -> Result<CommandCatalogueSnapshot, CommandHostError> {
        self.with_authority(|authority| authority.catalogue(caller))?
    }

    /// Loads one caller-authorized effective keymap.
    pub fn keymap(&self, caller: &str) -> Result<CommandKeymapLoadOutcome, CommandHostError> {
        self.with_authority(|authority| authority.keymap(caller))?
    }

    /// Previews one checked keymap patch.
    pub fn preview(
        &self,
        caller: &str,
        request: CommandKeymapPreview,
    ) -> Result<CommandKeymapPreviewResult, CommandHostError> {
        self.with_authority(|authority| authority.preview(caller, request))?
    }

    /// Commits one digest-bound keymap patch.
    pub fn commit(
        &self,
        caller: &str,
        request: CommandKeymapCommit,
    ) -> Result<CommandKeymapMutationResult, CommandHostError> {
        self.with_authority(|authority| authority.commit(caller, request))?
    }

    /// Resets caller-authorized keymap state.
    pub fn reset(
        &self,
        caller: &str,
        request: CommandKeymapReset,
    ) -> Result<CommandKeymapMutationResult, CommandHostError> {
        self.with_authority(|authority| authority.reset(caller, request))?
    }
}
