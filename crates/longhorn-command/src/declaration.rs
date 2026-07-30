use std::fmt;

use longhorn_core::{
    CommandCapabilityId, CommandCategoryId, CommandContextId, CommandId, CommandRouteId,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    CommandArgumentSchema, CommandRegistryError, CommandRegistryErrorCode, error::registry_error,
};

/// Bounded consumer-owned search keyword or phrase.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "string"))]
pub struct CommandKeyword(String);

impl CommandKeyword {
    const HARD_MAXIMUM_BYTES: usize = 65_536;

    /// Validates and constructs a keyword.
    pub fn new(value: impl Into<String>) -> Result<Self, CommandRegistryError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(registry_error(
                CommandRegistryErrorCode::EmptyText,
                "command keyword is empty",
            ));
        }
        if value.len() > Self::HARD_MAXIMUM_BYTES {
            return Err(registry_error(
                CommandRegistryErrorCode::TextTooLong,
                format!(
                    "command keyword contains {} bytes; hard maximum is {}",
                    value.len(),
                    Self::HARD_MAXIMUM_BYTES
                ),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the consumer-owned keyword text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandKeyword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for CommandKeyword {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CommandKeyword {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// One registered node in the finite command context tree.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandContextDefinition {
    /// Stable consumer-owned context identity.
    pub id: CommandContextId,
    /// Parent context, absent only for `global`.
    pub parent_id: Option<CommandContextId>,
}

/// One registered command composition capability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandCapabilityDefinition {
    /// Stable capability identity.
    pub id: CommandCapabilityId,
}

/// Renderer surface that may discover a command.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandSurface {
    /// Command palette.
    Palette,
    /// Application or context menu.
    Menu,
    /// Shortcut discovery and dispatch.
    Shortcut,
    /// Keybinding settings.
    Settings,
    /// Help and command reference.
    Help,
}

/// Explicit command visibility across shared discovery surfaces.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandVisibility {
    /// The command is intentionally absent from every discovery surface.
    pub hidden: bool,
    /// Visible in the command palette.
    pub palette: bool,
    /// Visible in menus.
    pub menu: bool,
    /// Eligible for shortcut discovery.
    pub shortcut: bool,
    /// Visible in keybinding settings.
    pub settings: bool,
    /// Visible in help.
    pub help: bool,
}

impl CommandVisibility {
    /// Explicit hidden visibility.
    pub const HIDDEN: Self = Self {
        hidden: true,
        palette: false,
        menu: false,
        shortcut: false,
        settings: false,
        help: false,
    };

    /// Visibility on every shared surface.
    pub const ALL: Self = Self {
        hidden: false,
        palette: true,
        menu: true,
        shortcut: true,
        settings: true,
        help: true,
    };

    /// Returns whether this declaration is visible on one surface.
    #[must_use]
    pub const fn contains(self, surface: CommandSurface) -> bool {
        if self.hidden {
            return false;
        }
        match surface {
            CommandSurface::Palette => self.palette,
            CommandSurface::Menu => self.menu,
            CommandSurface::Shortcut => self.shortcut,
            CommandSurface::Settings => self.settings,
            CommandSurface::Help => self.help,
        }
    }

    pub(crate) const fn is_valid(self) -> bool {
        let any_surface = self.palette || self.menu || self.shortcut || self.settings || self.help;
        (self.hidden && !any_surface) || (!self.hidden && any_surface)
    }
}

/// Whether a command may be admitted while editable text owns focus.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandTextInputPolicy {
    /// Block dispatch while editable text owns focus.
    Blocked,
    /// Permit dispatch while editable text owns focus.
    Allowed,
}

/// Declarative command metadata registered before registry seal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandDefinition {
    /// Stable semantic command identity.
    pub id: CommandId,
    /// Consumer-owned display label.
    pub label: String,
    /// Optional consumer-owned discovery description.
    pub description: Option<String>,
    /// Ordered discovery category path.
    pub category_path: Vec<CommandCategoryId>,
    /// Consumer-owned search keywords.
    pub keywords: Vec<CommandKeyword>,
    /// Optional consumer-owned icon resolver token.
    pub icon: Option<String>,
    /// Contexts in which the command may be considered.
    pub allowed_contexts: Vec<CommandContextId>,
    /// Capabilities required for registry composition.
    pub required_capabilities: Vec<CommandCapabilityId>,
    /// Shared discovery visibility.
    pub visibility: CommandVisibility,
    /// Editable-text admission posture.
    pub text_input_policy: CommandTextInputPolicy,
    /// Opaque route resolved by the consumer executor.
    pub route: CommandRouteId,
    /// Closed structural argument schema.
    pub arguments: CommandArgumentSchema,
}
