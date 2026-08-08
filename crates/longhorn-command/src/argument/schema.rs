use std::collections::{BTreeMap, BTreeSet};

use longhorn_core::{CommandEnumValueId, CommandFieldId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    CommandArgumentError, CommandArgumentErrorCode, CommandLimits, CommandRegistryError,
    CommandRegistryErrorCode,
    error::{argument_error, registry_error},
};

use super::{CommandFiniteNumber, validate_kind, validate_object, validate_typed_value};

/// Closed primitive kind accepted by one command argument field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "camelCase"))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum CommandArgumentKind {
    /// Boolean value.
    Boolean,
    /// Finite floating-point value with optional inclusive bounds.
    Number {
        /// Optional inclusive minimum.
        minimum: Option<CommandFiniteNumber>,
        /// Optional inclusive maximum.
        maximum: Option<CommandFiniteNumber>,
    },
    /// Signed integer with optional inclusive bounds.
    Integer {
        /// Optional inclusive minimum.
        #[cfg_attr(feature = "bindings", ts(type = "number | null"))]
        minimum: Option<i64>,
        /// Optional inclusive maximum.
        #[cfg_attr(feature = "bindings", ts(type = "number | null"))]
        maximum: Option<i64>,
    },
    /// UTF-8 string bounded by encoded byte length.
    String {
        /// Maximum UTF-8 bytes.
        maximum_bytes: usize,
    },
    /// Closed string enum.
    Enum {
        /// Admitted values.
        values: Vec<CommandEnumValueId>,
    },
}

/// One named field in a bounded object argument schema.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandArgumentField {
    /// Stable field identity and serialized object key.
    pub id: CommandFieldId,
    /// Whether omission is invalid when no default exists.
    pub required: bool,
    /// Optional normalized default.
    pub default: Option<CommandArgumentValue>,
    /// Closed primitive field kind.
    pub kind: CommandArgumentKind,
}

/// Closed v1 command argument schema.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "camelCase"))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "shape"
)]
pub enum CommandArgumentSchema {
    /// The command accepts only JSON `null`.
    None,
    /// The command accepts one object containing declared primitive fields.
    Object {
        /// Named object fields.
        fields: Vec<CommandArgumentField>,
    },
}

/// One normalized primitive command argument.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(rename_all = "camelCase"))]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    tag = "kind",
    content = "value"
)]
pub enum CommandArgumentValue {
    /// Boolean value.
    Boolean(bool),
    /// Finite floating-point value.
    Number(CommandFiniteNumber),
    /// Signed integer.
    Integer(#[cfg_attr(feature = "bindings", ts(type = "number"))] i64),
    /// Bounded UTF-8 string.
    String(String),
    /// Closed enum value.
    Enum(CommandEnumValueId),
}

/// Structurally validated and deterministically ordered command arguments.
#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "bindings",
    ts(type = "Record<CommandFieldId, CommandArgumentValue>")
)]
#[serde(transparent)]
pub struct CommandArguments(pub(crate) BTreeMap<CommandFieldId, CommandArgumentValue>);

impl CommandArguments {
    /// Returns normalized fields in stable id order.
    pub fn fields(
        &self,
    ) -> impl ExactSizeIterator<Item = (&CommandFieldId, &CommandArgumentValue)> {
        self.0.iter()
    }

    /// Returns one normalized field.
    #[must_use]
    pub fn get(&self, id: &CommandFieldId) -> Option<&CommandArgumentValue> {
        self.0.get(id)
    }
}

impl CommandArgumentSchema {
    /// Validates raw JSON structurally and returns one normalized bounded value.
    pub fn validate(&self, input: &Value) -> Result<CommandArguments, CommandArgumentError> {
        match self {
            Self::None => {
                if input.is_null() {
                    Ok(CommandArguments::default())
                } else {
                    Err(argument_error(
                        CommandArgumentErrorCode::ArgumentsNotAllowed,
                        None,
                        "command does not accept arguments; expected null",
                    ))
                }
            }
            Self::Object { fields } => validate_object(fields, input),
        }
    }

    pub(crate) fn validate_definition(
        &self,
        limits: CommandLimits,
    ) -> Result<(), CommandRegistryError> {
        let Self::Object { fields } = self else {
            return Ok(());
        };
        if fields.is_empty() {
            return Err(registry_error(
                CommandRegistryErrorCode::InvalidArgumentSchema,
                "object argument schema must declare at least one field",
            ));
        }
        if fields.len() > limits.maximum_fields_per_command {
            return Err(registry_error(
                CommandRegistryErrorCode::LimitExceeded,
                format!(
                    "argument schema declares {} fields; maximum is {}",
                    fields.len(),
                    limits.maximum_fields_per_command
                ),
            ));
        }
        let mut ids = BTreeSet::new();
        for field in fields {
            if !ids.insert(field.id.clone()) {
                return Err(registry_error(
                    CommandRegistryErrorCode::DuplicateItem,
                    format!("duplicate argument field {}", field.id),
                ));
            }
            validate_kind(field, limits)?;
            if let Some(default) = &field.default {
                validate_typed_value(&field.id, &field.kind, default).map_err(|error| {
                    registry_error(
                        CommandRegistryErrorCode::InvalidArgumentDefault,
                        format!("invalid default for field {}: {}", field.id, error.detail()),
                    )
                })?;
            }
        }
        Ok(())
    }

    pub(crate) fn canonicalize(&mut self) {
        if let Self::Object { fields } = self {
            fields.sort_by(|left, right| left.id.cmp(&right.id));
        }
    }
}
