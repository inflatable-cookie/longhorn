use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, de};

const HARD_MAXIMUM_SMALL: usize = 4_096;
const HARD_MAXIMUM_LABEL_BYTES: usize = 16_384;
const HARD_MAXIMUM_INSTANCES: usize = 65_536;

/// Explicit finite bounds for one Surface registry, document, and resolution input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct SurfaceLimits {
    maximum_schemas: usize,
    maximum_regions_per_schema: usize,
    maximum_sizing_slots_per_schema: usize,
    maximum_panel_definitions: usize,
    maximum_surfaces: usize,
    maximum_windows: usize,
    maximum_host_preferences_per_surface: usize,
    maximum_label_bytes: usize,
    maximum_panel_instances: usize,
    maximum_panel_instances_per_region: usize,
}

/// Builder for [`SurfaceLimits`] that validates every bound in [`Self::build`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceLimitsBuilder {
    maximum_schemas: usize,
    maximum_regions_per_schema: usize,
    maximum_sizing_slots_per_schema: usize,
    maximum_panel_definitions: usize,
    maximum_surfaces: usize,
    maximum_windows: usize,
    maximum_host_preferences_per_surface: usize,
    maximum_label_bytes: usize,
    maximum_panel_instances: usize,
    maximum_panel_instances_per_region: usize,
}

impl Default for SurfaceLimitsBuilder {
    fn default() -> Self {
        Self {
            maximum_schemas: 16,
            maximum_regions_per_schema: 32,
            maximum_sizing_slots_per_schema: 32,
            maximum_panel_definitions: 64,
            maximum_surfaces: 64,
            maximum_windows: 16,
            maximum_host_preferences_per_surface: 16,
            maximum_label_bytes: 256,
            maximum_panel_instances: 256,
            maximum_panel_instances_per_region: 64,
        }
    }
}

impl SurfaceLimitsBuilder {
    /// Sets the maximum registered schemas.
    #[must_use]
    pub const fn maximum_schemas(mut self, value: usize) -> Self {
        self.maximum_schemas = value;
        self
    }

    /// Sets the maximum regions in one schema.
    #[must_use]
    pub const fn maximum_regions_per_schema(mut self, value: usize) -> Self {
        self.maximum_regions_per_schema = value;
        self
    }

    /// Sets the maximum sizing slots in one schema.
    #[must_use]
    pub const fn maximum_sizing_slots_per_schema(mut self, value: usize) -> Self {
        self.maximum_sizing_slots_per_schema = value;
        self
    }

    /// Sets the maximum registered panel definitions.
    #[must_use]
    pub const fn maximum_panel_definitions(mut self, value: usize) -> Self {
        self.maximum_panel_definitions = value;
        self
    }

    /// Sets the maximum Surface records in one document.
    #[must_use]
    pub const fn maximum_surfaces(mut self, value: usize) -> Self {
        self.maximum_surfaces = value;
        self
    }

    /// Sets the maximum participating windows in one document or input.
    #[must_use]
    pub const fn maximum_windows(mut self, value: usize) -> Self {
        self.maximum_windows = value;
        self
    }

    /// Sets the maximum ordered host preferences for one Surface.
    #[must_use]
    pub const fn maximum_host_preferences_per_surface(mut self, value: usize) -> Self {
        self.maximum_host_preferences_per_surface = value;
        self
    }

    /// Sets the maximum UTF-8 byte length of one optional label.
    #[must_use]
    pub const fn maximum_label_bytes(mut self, value: usize) -> Self {
        self.maximum_label_bytes = value;
        self
    }

    /// Sets the maximum panel instances in one document.
    #[must_use]
    pub const fn maximum_panel_instances(mut self, value: usize) -> Self {
        self.maximum_panel_instances = value;
        self
    }

    /// Sets the maximum panel instances in one region.
    #[must_use]
    pub const fn maximum_panel_instances_per_region(mut self, value: usize) -> Self {
        self.maximum_panel_instances_per_region = value;
        self
    }

    /// Validates all configured bounds and constructs [`SurfaceLimits`].
    ///
    /// # Errors
    ///
    /// Returns [`SurfaceLimitsError::Zero`] if any bound is zero, or
    /// [`SurfaceLimitsError::ExceedsHardMaximum`] if any bound exceeds its
    /// library hard ceiling.
    pub fn build(self) -> Result<SurfaceLimits, SurfaceLimitsError> {
        let values = [
            ("maximum_schemas", self.maximum_schemas, HARD_MAXIMUM_SMALL),
            (
                "maximum_regions_per_schema",
                self.maximum_regions_per_schema,
                HARD_MAXIMUM_SMALL,
            ),
            (
                "maximum_sizing_slots_per_schema",
                self.maximum_sizing_slots_per_schema,
                HARD_MAXIMUM_SMALL,
            ),
            (
                "maximum_panel_definitions",
                self.maximum_panel_definitions,
                HARD_MAXIMUM_SMALL,
            ),
            (
                "maximum_surfaces",
                self.maximum_surfaces,
                HARD_MAXIMUM_SMALL,
            ),
            ("maximum_windows", self.maximum_windows, HARD_MAXIMUM_SMALL),
            (
                "maximum_host_preferences_per_surface",
                self.maximum_host_preferences_per_surface,
                HARD_MAXIMUM_SMALL,
            ),
            (
                "maximum_label_bytes",
                self.maximum_label_bytes,
                HARD_MAXIMUM_LABEL_BYTES,
            ),
            (
                "maximum_panel_instances",
                self.maximum_panel_instances,
                HARD_MAXIMUM_INSTANCES,
            ),
            (
                "maximum_panel_instances_per_region",
                self.maximum_panel_instances_per_region,
                HARD_MAXIMUM_INSTANCES,
            ),
        ];

        for (name, actual, maximum) in values {
            if actual == 0 {
                return Err(SurfaceLimitsError::Zero { name });
            }
            if actual > maximum {
                return Err(SurfaceLimitsError::ExceedsHardMaximum {
                    name,
                    maximum,
                    actual,
                });
            }
        }

        Ok(SurfaceLimits {
            maximum_schemas: self.maximum_schemas,
            maximum_regions_per_schema: self.maximum_regions_per_schema,
            maximum_sizing_slots_per_schema: self.maximum_sizing_slots_per_schema,
            maximum_panel_definitions: self.maximum_panel_definitions,
            maximum_surfaces: self.maximum_surfaces,
            maximum_windows: self.maximum_windows,
            maximum_host_preferences_per_surface: self.maximum_host_preferences_per_surface,
            maximum_label_bytes: self.maximum_label_bytes,
            maximum_panel_instances: self.maximum_panel_instances,
            maximum_panel_instances_per_region: self.maximum_panel_instances_per_region,
        })
    }
}

impl<'de> Deserialize<'de> for SurfaceLimits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SerializedLimits {
            maximum_schemas: usize,
            maximum_regions_per_schema: usize,
            maximum_sizing_slots_per_schema: usize,
            maximum_panel_definitions: usize,
            maximum_surfaces: usize,
            maximum_windows: usize,
            maximum_host_preferences_per_surface: usize,
            maximum_label_bytes: usize,
            maximum_panel_instances: usize,
            maximum_panel_instances_per_region: usize,
        }

        let limits = SerializedLimits::deserialize(deserializer)?;
        Self::builder()
            .maximum_schemas(limits.maximum_schemas)
            .maximum_regions_per_schema(limits.maximum_regions_per_schema)
            .maximum_sizing_slots_per_schema(limits.maximum_sizing_slots_per_schema)
            .maximum_panel_definitions(limits.maximum_panel_definitions)
            .maximum_surfaces(limits.maximum_surfaces)
            .maximum_windows(limits.maximum_windows)
            .maximum_host_preferences_per_surface(limits.maximum_host_preferences_per_surface)
            .maximum_label_bytes(limits.maximum_label_bytes)
            .maximum_panel_instances(limits.maximum_panel_instances)
            .maximum_panel_instances_per_region(limits.maximum_panel_instances_per_region)
            .build()
            .map_err(de::Error::custom)
    }
}

impl SurfaceLimits {
    /// Starts a builder for explicit finite Surface and layout bounds.
    #[must_use]
    pub fn builder() -> SurfaceLimitsBuilder {
        SurfaceLimitsBuilder::default()
    }

    /// Returns the maximum registered schemas.
    #[must_use]
    pub const fn maximum_schemas(self) -> usize {
        self.maximum_schemas
    }

    /// Returns the maximum regions in one schema.
    #[must_use]
    pub const fn maximum_regions_per_schema(self) -> usize {
        self.maximum_regions_per_schema
    }

    /// Returns the maximum sizing slots in one schema.
    #[must_use]
    pub const fn maximum_sizing_slots_per_schema(self) -> usize {
        self.maximum_sizing_slots_per_schema
    }

    /// Returns the maximum registered panel definitions.
    #[must_use]
    pub const fn maximum_panel_definitions(self) -> usize {
        self.maximum_panel_definitions
    }

    /// Returns the maximum Surface records in one document.
    #[must_use]
    pub const fn maximum_surfaces(self) -> usize {
        self.maximum_surfaces
    }

    /// Returns the maximum participating windows in one document or input.
    #[must_use]
    pub const fn maximum_windows(self) -> usize {
        self.maximum_windows
    }

    /// Returns the maximum ordered host preferences for one Surface.
    #[must_use]
    pub const fn maximum_host_preferences_per_surface(self) -> usize {
        self.maximum_host_preferences_per_surface
    }

    /// Returns the maximum UTF-8 byte length of one optional label.
    #[must_use]
    pub const fn maximum_label_bytes(self) -> usize {
        self.maximum_label_bytes
    }

    /// Returns the maximum panel instances in one document.
    #[must_use]
    pub const fn maximum_panel_instances(self) -> usize {
        self.maximum_panel_instances
    }

    /// Returns the maximum panel instances in one region.
    #[must_use]
    pub const fn maximum_panel_instances_per_region(self) -> usize {
        self.maximum_panel_instances_per_region
    }
}

/// Invalid finite Surface bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceLimitsError {
    /// A required limit was zero.
    Zero {
        /// Stable limit field name.
        name: &'static str,
    },
    /// A limit exceeded its library hard ceiling.
    ExceedsHardMaximum {
        /// Stable limit field name.
        name: &'static str,
        /// Library hard ceiling.
        maximum: usize,
        /// Rejected caller value.
        actual: usize,
    },
}

impl fmt::Display for SurfaceLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero { name } => write!(formatter, "{name} must be nonzero"),
            Self::ExceedsHardMaximum {
                name,
                maximum,
                actual,
            } => write!(
                formatter,
                "{name} is {actual}; library hard maximum is {maximum}"
            ),
        }
    }
}

impl Error for SurfaceLimitsError {}
