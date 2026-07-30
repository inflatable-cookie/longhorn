use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const MAXIMUM_PHYSICAL_CODE_BYTES: usize = 64;

/// Desktop platform used to resolve semantic modifiers and platform bindings.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandPlatform {
    /// macOS.
    MacOs,
    /// Windows.
    Windows,
    /// Linux.
    Linux,
}

impl CommandPlatform {
    /// Every supported desktop platform in stable order.
    pub const ALL: [Self; 3] = [Self::MacOs, Self::Windows, Self::Linux];
}

/// Platform posture declared by one preset binding.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandPlatformScope {
    /// Binding applies to every supported desktop platform.
    Any,
    /// Binding applies only to macOS.
    MacOs,
    /// Binding applies only to Windows.
    Windows,
    /// Binding applies only to Linux.
    Linux,
}

impl CommandPlatformScope {
    /// Returns whether the binding applies on the supplied runtime platform.
    #[must_use]
    pub const fn includes(self, platform: CommandPlatform) -> bool {
        matches!(
            (self, platform),
            (Self::Any, _)
                | (Self::MacOs, CommandPlatform::MacOs)
                | (Self::Windows, CommandPlatform::Windows)
                | (Self::Linux, CommandPlatform::Linux)
        )
    }

    pub(crate) fn platforms(self) -> impl Iterator<Item = CommandPlatform> {
        CommandPlatform::ALL
            .into_iter()
            .filter(move |platform| self.includes(*platform))
    }
}

/// Physical key identity sourced from `KeyboardEvent.code`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "string"))]
pub struct CommandPhysicalCode(String);

impl CommandPhysicalCode {
    /// Validates one bounded physical code.
    pub fn new(value: impl Into<String>) -> Result<Self, CommandPhysicalCodeError> {
        let value = value.into();
        if value.is_empty() {
            return Err(CommandPhysicalCodeError::Empty);
        }
        if value.len() > MAXIMUM_PHYSICAL_CODE_BYTES {
            return Err(CommandPhysicalCodeError::TooLong {
                maximum: MAXIMUM_PHYSICAL_CODE_BYTES,
                actual: value.len(),
            });
        }
        if value == "Unidentified" {
            return Err(CommandPhysicalCodeError::Unidentified);
        }
        if let Some((index, _)) = value
            .char_indices()
            .find(|(_, character)| !character.is_ascii_alphanumeric())
        {
            return Err(CommandPhysicalCodeError::InvalidCharacter { index });
        }
        if !value.as_bytes()[0].is_ascii_uppercase() {
            return Err(CommandPhysicalCodeError::InvalidStart);
        }
        Ok(Self(value))
    }

    /// Returns the serialized DOM physical code.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandPhysicalCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for CommandPhysicalCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CommandPhysicalCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Invalid physical keyboard code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandPhysicalCodeError {
    /// Code is empty.
    Empty,
    /// Code is the DOM unknown-key sentinel.
    Unidentified,
    /// Code does not begin with the uppercase ASCII used by DOM code values.
    InvalidStart,
    /// Code exceeds the defensive byte limit.
    TooLong {
        /// Maximum accepted bytes.
        maximum: usize,
        /// Supplied bytes.
        actual: usize,
    },
    /// Code contains a character outside the DOM code grammar used by v1.
    InvalidCharacter {
        /// Invalid UTF-8 byte offset.
        index: usize,
    },
}

impl fmt::Display for CommandPhysicalCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("physical keyboard code is empty"),
            Self::Unidentified => formatter.write_str("physical keyboard code is unidentified"),
            Self::InvalidStart => {
                formatter.write_str("physical keyboard code must begin with uppercase ASCII")
            }
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "physical keyboard code contains {actual} bytes; maximum is {maximum}"
            ),
            Self::InvalidCharacter { index } => write!(
                formatter,
                "physical keyboard code contains an invalid character at byte {index}"
            ),
        }
    }
}

impl Error for CommandPhysicalCodeError {}

/// Canonical native modifier state in Control, Alt, Shift, Meta order.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandModifiers {
    /// Control is pressed.
    pub control: bool,
    /// Alt or Option is pressed.
    pub alt: bool,
    /// Shift is pressed.
    pub shift: bool,
    /// Meta or Command is pressed.
    pub meta: bool,
}

/// Preset modifier state with one semantic primary modifier.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandTriggerModifiers {
    /// Meta on macOS; Control on Windows and Linux.
    pub primary: bool,
    /// Explicit Control modifier.
    pub control: bool,
    /// Explicit Alt or Option modifier.
    pub alt: bool,
    /// Explicit Shift modifier.
    pub shift: bool,
    /// Explicit Meta or Command modifier.
    pub meta: bool,
}

impl CommandTriggerModifiers {
    /// Resolves semantic primary into one canonical native modifier state.
    pub fn resolve(
        self,
        platform: CommandPlatform,
    ) -> Result<CommandModifiers, CommandModifierError> {
        if self.primary
            && matches!(
                platform,
                CommandPlatform::MacOs if self.meta
            )
        {
            return Err(CommandModifierError::DuplicatePrimary {
                platform,
                modifier: CommandNativeModifier::Meta,
            });
        }
        if self.primary
            && matches!(
                platform,
                CommandPlatform::Windows | CommandPlatform::Linux if self.control
            )
        {
            return Err(CommandModifierError::DuplicatePrimary {
                platform,
                modifier: CommandNativeModifier::Control,
            });
        }

        Ok(CommandModifiers {
            control: self.control
                || (self.primary
                    && matches!(platform, CommandPlatform::Windows | CommandPlatform::Linux)),
            alt: self.alt,
            shift: self.shift,
            meta: self.meta || (self.primary && platform == CommandPlatform::MacOs),
        })
    }
}

/// Native modifier named by a normalization error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandNativeModifier {
    /// Control.
    Control,
    /// Meta or Command.
    Meta,
}

/// Invalid semantic modifier declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandModifierError {
    /// Semantic primary repeats its platform-native modifier explicitly.
    DuplicatePrimary {
        /// Platform on which the duplication occurs.
        platform: CommandPlatform,
        /// Duplicated native modifier.
        modifier: CommandNativeModifier,
    },
}

impl fmt::Display for CommandModifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePrimary { platform, modifier } => write!(
                formatter,
                "semantic primary duplicates explicit {modifier:?} on {platform:?}"
            ),
        }
    }
}

impl Error for CommandModifierError {}

/// One press-only physical trigger declared by a preset or override.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeyTrigger {
    /// Physical DOM code.
    pub code: CommandPhysicalCode,
    /// Semantic and explicit modifier declaration.
    pub modifiers: CommandTriggerModifiers,
}

impl CommandKeyTrigger {
    /// Resolves this trigger for one runtime platform.
    pub fn resolve(
        &self,
        platform: CommandPlatform,
    ) -> Result<CommandKeyChord, CommandModifierError> {
        Ok(CommandKeyChord {
            code: self.code.clone(),
            modifiers: self.modifiers.resolve(platform)?,
        })
    }
}

/// One canonical native physical chord.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeyChord {
    /// Physical DOM code.
    pub code: CommandPhysicalCode,
    /// Canonical native modifier state.
    pub modifiers: CommandModifiers,
}

impl CommandKeyChord {
    /// Produces one deterministic platform shortcut label.
    #[must_use]
    pub fn label(&self, platform: CommandPlatform) -> String {
        let key = key_label(&self.code);
        match platform {
            CommandPlatform::MacOs => {
                let mut label = String::new();
                if self.modifiers.control {
                    label.push('⌃');
                }
                if self.modifiers.alt {
                    label.push('⌥');
                }
                if self.modifiers.shift {
                    label.push('⇧');
                }
                if self.modifiers.meta {
                    label.push('⌘');
                }
                label.push_str(key);
                label
            }
            CommandPlatform::Windows | CommandPlatform::Linux => {
                let mut parts = Vec::with_capacity(5);
                if self.modifiers.control {
                    parts.push("Ctrl");
                }
                if self.modifiers.alt {
                    parts.push("Alt");
                }
                if self.modifiers.shift {
                    parts.push("Shift");
                }
                if self.modifiers.meta {
                    parts.push("Meta");
                }
                parts.push(key);
                parts.join("+")
            }
        }
    }
}

fn key_label(code: &CommandPhysicalCode) -> &str {
    match code.as_str() {
        value if value.len() == 4 && value.starts_with("Key") => &value[3..],
        value if value.len() == 6 && value.starts_with("Digit") => &value[5..],
        "ArrowUp" => "↑",
        "ArrowDown" => "↓",
        "ArrowLeft" => "←",
        "ArrowRight" => "→",
        "Backspace" => "Backspace",
        "Delete" => "Delete",
        "Enter" => "Enter",
        "Escape" => "Esc",
        "Space" => "Space",
        "Tab" => "Tab",
        value => value,
    }
}

/// Browser-independent facts for one press event.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandKeyboardInput {
    /// Canonical physical chord.
    pub chord: CommandKeyChord,
    /// Whether the platform marked this press as repeated.
    pub repeat: bool,
    /// Whether IME or composition owns this press.
    pub composing: bool,
    /// Whether editable text currently owns focus.
    pub editable_text: bool,
}

/// Keyboard resolver mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum CommandKeyboardMode {
    /// Resolve and dispatch an effective binding.
    Dispatch,
    /// Record a non-reserved chord without dispatch.
    Capture,
}

/// Consumer-injected platform reservation policy.
pub trait CommandReservedChordPolicy {
    /// Returns whether the platform or application shell reserves this chord.
    fn is_reserved(&self, platform: CommandPlatform, chord: &CommandKeyChord) -> bool;
}

/// Policy that reserves no chords.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoReservedCommandChords;

impl CommandReservedChordPolicy for NoReservedCommandChords {
    fn is_reserved(&self, _platform: CommandPlatform, _chord: &CommandKeyChord) -> bool {
        false
    }
}
