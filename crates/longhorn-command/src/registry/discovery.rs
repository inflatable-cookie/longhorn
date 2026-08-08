//! Discovery projection for sealed commands.

use longhorn_core::CommandId;
use serde::{Deserialize, Serialize};

use crate::{CommandDefinition, CommandTextInputPolicy, CommandVisibility};

/// Stable framework-neutral discovery projection for one command.
/// Stable framework-neutral discovery projection for one command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandDiscoveryRecord {
    /// Stable semantic command identity.
    pub id: CommandId,
    /// Consumer-owned display label.
    pub label: String,
    /// Optional consumer-owned description.
    pub description: Option<String>,
    /// Ordered discovery category path.
    pub category_path: Vec<longhorn_core::CommandCategoryId>,
    /// Consumer-owned search keywords.
    pub keywords: Vec<crate::CommandKeyword>,
    /// Optional consumer-owned icon resolver token.
    pub icon: Option<String>,
    /// Shared discovery visibility.
    pub visibility: CommandVisibility,
    /// Editable-text admission posture.
    pub text_input_policy: CommandTextInputPolicy,
    /// Closed structural argument schema.
    pub arguments: crate::CommandArgumentSchema,
}

impl From<&CommandDefinition> for CommandDiscoveryRecord {
    fn from(command: &CommandDefinition) -> Self {
        Self {
            id: command.id.clone(),
            label: command.label.clone(),
            description: command.description.clone(),
            category_path: command.category_path.clone(),
            keywords: command.keywords.clone(),
            icon: command.icon.clone(),
            visibility: command.visibility,
            text_input_policy: command.text_input_policy,
            arguments: command.arguments.clone(),
        }
    }
}
