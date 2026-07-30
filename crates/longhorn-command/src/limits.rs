use serde::{Deserialize, Serialize};

/// Explicit defensive limits for one command registry.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandLimits {
    /// Maximum registered commands.
    pub maximum_commands: usize,
    /// Maximum registered contexts.
    pub maximum_contexts: usize,
    /// Maximum registered capabilities.
    pub maximum_capabilities: usize,
    /// Maximum context-tree depth including `global`.
    pub maximum_context_depth: usize,
    /// Maximum categories declared by one command.
    pub maximum_categories_per_command: usize,
    /// Maximum keywords declared by one command.
    pub maximum_keywords_per_command: usize,
    /// Maximum allowed contexts declared by one command.
    pub maximum_contexts_per_command: usize,
    /// Maximum required capabilities declared by one command.
    pub maximum_capabilities_per_command: usize,
    /// Maximum fields declared by one command.
    pub maximum_fields_per_command: usize,
    /// Maximum values in one closed enum field.
    pub maximum_enum_values_per_field: usize,
    /// Maximum bytes in a label or icon token.
    pub maximum_label_bytes: usize,
    /// Maximum bytes in a description.
    pub maximum_description_bytes: usize,
    /// Maximum bytes in one keyword.
    pub maximum_keyword_bytes: usize,
    /// Maximum bytes in a validated string argument.
    pub maximum_argument_string_bytes: usize,
    /// Maximum bytes in one search query.
    pub maximum_search_query_bytes: usize,
}

impl CommandLimits {
    const HARD_MAXIMUM_REGISTRATIONS: usize = 65_536;
    const HARD_MAXIMUM_ITEMS: usize = 4_096;
    const HARD_MAXIMUM_TEXT_BYTES: usize = 65_536;
    const HARD_MAXIMUM_DEPTH: usize = 256;

    /// Returns whether every limit is nonzero and below its defensive ceiling.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.maximum_commands > 0
            && self.maximum_commands <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_contexts > 0
            && self.maximum_contexts <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_capabilities > 0
            && self.maximum_capabilities <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_context_depth > 0
            && self.maximum_context_depth <= Self::HARD_MAXIMUM_DEPTH
            && self.maximum_categories_per_command > 0
            && self.maximum_categories_per_command <= Self::HARD_MAXIMUM_ITEMS
            && self.maximum_keywords_per_command > 0
            && self.maximum_keywords_per_command <= Self::HARD_MAXIMUM_ITEMS
            && self.maximum_contexts_per_command > 0
            && self.maximum_contexts_per_command <= Self::HARD_MAXIMUM_ITEMS
            && self.maximum_capabilities_per_command > 0
            && self.maximum_capabilities_per_command <= Self::HARD_MAXIMUM_ITEMS
            && self.maximum_fields_per_command > 0
            && self.maximum_fields_per_command <= Self::HARD_MAXIMUM_ITEMS
            && self.maximum_enum_values_per_field > 0
            && self.maximum_enum_values_per_field <= Self::HARD_MAXIMUM_ITEMS
            && self.maximum_label_bytes > 0
            && self.maximum_label_bytes <= Self::HARD_MAXIMUM_TEXT_BYTES
            && self.maximum_description_bytes > 0
            && self.maximum_description_bytes <= Self::HARD_MAXIMUM_TEXT_BYTES
            && self.maximum_keyword_bytes > 0
            && self.maximum_keyword_bytes <= Self::HARD_MAXIMUM_TEXT_BYTES
            && self.maximum_argument_string_bytes > 0
            && self.maximum_argument_string_bytes <= Self::HARD_MAXIMUM_TEXT_BYTES
            && self.maximum_search_query_bytes > 0
            && self.maximum_search_query_bytes <= Self::HARD_MAXIMUM_TEXT_BYTES
    }
}

impl Default for CommandLimits {
    fn default() -> Self {
        Self {
            maximum_commands: 8_192,
            maximum_contexts: 1_024,
            maximum_capabilities: 1_024,
            maximum_context_depth: 64,
            maximum_categories_per_command: 16,
            maximum_keywords_per_command: 64,
            maximum_contexts_per_command: 64,
            maximum_capabilities_per_command: 64,
            maximum_fields_per_command: 64,
            maximum_enum_values_per_field: 256,
            maximum_label_bytes: 1_024,
            maximum_description_bytes: 8_192,
            maximum_keyword_bytes: 256,
            maximum_argument_string_bytes: 4_096,
            maximum_search_query_bytes: 1_024,
        }
    }
}
