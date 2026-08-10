use serde::{Deserialize, Serialize};

/// Explicit limits for one settings registry and its opaque values.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SettingsLimits {
    /// Maximum registered modules.
    pub maximum_modules: usize,
    /// Maximum registered sections.
    pub maximum_sections: usize,
    /// Maximum registered pages.
    pub maximum_pages: usize,
    /// Maximum registered renderer keys.
    pub maximum_renderers: usize,
    /// Maximum registered scopes.
    pub maximum_scopes: usize,
    /// Maximum registered apply units.
    pub maximum_apply_units: usize,
    /// Maximum registered capabilities.
    pub maximum_capabilities: usize,
    /// Maximum anchors declared by one page.
    pub maximum_anchors_per_page: usize,
    /// Maximum search keywords declared by one page.
    pub maximum_keywords_per_page: usize,
    /// Maximum bytes in a label.
    pub maximum_label_bytes: usize,
    /// Maximum bytes in one search keyword.
    pub maximum_keyword_bytes: usize,
    /// Maximum serialized bytes in one opaque value envelope.
    pub maximum_opaque_value_bytes: usize,
}

impl SettingsLimits {
    /// Defensive ceiling on any one registration count.
    pub const HARD_MAXIMUM_REGISTRATIONS: usize = 65_536;
    /// Defensive ceiling on items in one page.
    pub const HARD_MAXIMUM_PAGE_ITEMS: usize = 4_096;
    /// Defensive ceiling on bytes in any bounded text field.
    pub const HARD_MAXIMUM_TEXT_BYTES: usize = 16_384;
    /// Defensive ceiling on one opaque value envelope.
    pub const HARD_MAXIMUM_OPAQUE_VALUE_BYTES: usize = 1_048_576;

    /// Returns whether every limit is nonzero and below the defensive ceiling.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.maximum_modules > 0
            && self.maximum_modules <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_sections > 0
            && self.maximum_sections <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_pages > 0
            && self.maximum_pages <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_renderers > 0
            && self.maximum_renderers <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_scopes > 0
            && self.maximum_scopes <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_apply_units > 0
            && self.maximum_apply_units <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_capabilities > 0
            && self.maximum_capabilities <= Self::HARD_MAXIMUM_REGISTRATIONS
            && self.maximum_anchors_per_page > 0
            && self.maximum_anchors_per_page <= Self::HARD_MAXIMUM_PAGE_ITEMS
            && self.maximum_keywords_per_page > 0
            && self.maximum_keywords_per_page <= Self::HARD_MAXIMUM_PAGE_ITEMS
            && self.maximum_label_bytes > 0
            && self.maximum_label_bytes <= Self::HARD_MAXIMUM_TEXT_BYTES
            && self.maximum_keyword_bytes > 0
            && self.maximum_keyword_bytes <= Self::HARD_MAXIMUM_TEXT_BYTES
            && self.maximum_opaque_value_bytes > 0
            && self.maximum_opaque_value_bytes <= Self::HARD_MAXIMUM_OPAQUE_VALUE_BYTES
    }
}

impl Default for SettingsLimits {
    fn default() -> Self {
        Self {
            maximum_modules: 128,
            maximum_sections: 512,
            maximum_pages: 2_048,
            maximum_renderers: 512,
            maximum_scopes: 2_048,
            maximum_apply_units: 2_048,
            maximum_capabilities: 512,
            maximum_anchors_per_page: 128,
            maximum_keywords_per_page: 128,
            maximum_label_bytes: 1_024,
            maximum_keyword_bytes: 256,
            maximum_opaque_value_bytes: 65_536,
        }
    }
}
