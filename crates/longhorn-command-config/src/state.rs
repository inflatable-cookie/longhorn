use longhorn_command::CommandKeymapOverride;
use longhorn_core::CommandKeymapPresetId;
use serde::{Deserialize, Serialize};

use crate::CommandKeymapRevision;

/// Current persisted active preset and sparse override state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeymapState {
    /// Monotonic authoritative state revision.
    pub revision: CommandKeymapRevision,
    /// Active immutable preset.
    pub active_preset_id: CommandKeymapPresetId,
    /// Sparse disable, replace, and add directives.
    pub overrides: Vec<CommandKeymapOverride>,
}

impl CommandKeymapState {
    /// Constructs initial state for one active preset.
    #[must_use]
    pub const fn initial(active_preset_id: CommandKeymapPresetId) -> Self {
        Self {
            revision: CommandKeymapRevision::INITIAL,
            active_preset_id,
            overrides: Vec::new(),
        }
    }
}
