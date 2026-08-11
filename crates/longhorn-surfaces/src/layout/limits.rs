use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, de};

const HARD_MAXIMUM_SMALL: usize = 4_096;
const HARD_MAXIMUM_INSTANCES: usize = 65_536;

/// Explicit finite bounds for one layout definition registry and document.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct LayoutLimits {
    maximum_schemas: usize,
    maximum_regions_per_schema: usize,
    maximum_sizing_slots_per_schema: usize,
    maximum_panel_definitions: usize,
    // Card 179 note: SurfaceLimits bounds this same list. Two limits on one
    // list can disagree, and only the stricter one takes effect. Collapsing
    // them means changing this constructor's arity, which is a wider change
    // than the rename that surfaced it.
    maximum_surfaces: usize,
    maximum_panel_instances: usize,
    maximum_panel_instances_per_region: usize,
}

impl<'de> Deserialize<'de> for LayoutLimits {
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
            maximum_panel_instances: usize,
            maximum_panel_instances_per_region: usize,
        }

        let limits = SerializedLimits::deserialize(deserializer)?;
        Self::new(
            limits.maximum_schemas,
            limits.maximum_regions_per_schema,
            limits.maximum_sizing_slots_per_schema,
            limits.maximum_panel_definitions,
            limits.maximum_surfaces,
            limits.maximum_panel_instances,
            limits.maximum_panel_instances_per_region,
        )
        .map_err(de::Error::custom)
    }
}

impl LayoutLimits {
    /// Constructs limits and rejects zero or excessive values.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        maximum_schemas: usize,
        maximum_regions_per_schema: usize,
        maximum_sizing_slots_per_schema: usize,
        maximum_panel_definitions: usize,
        maximum_surfaces: usize,
        maximum_panel_instances: usize,
        maximum_panel_instances_per_region: usize,
    ) -> Result<Self, LayoutLimitsError> {
        let values = [
            ("maximum_schemas", maximum_schemas, HARD_MAXIMUM_SMALL),
            (
                "maximum_regions_per_schema",
                maximum_regions_per_schema,
                HARD_MAXIMUM_SMALL,
            ),
            (
                "maximum_sizing_slots_per_schema",
                maximum_sizing_slots_per_schema,
                HARD_MAXIMUM_SMALL,
            ),
            (
                "maximum_panel_definitions",
                maximum_panel_definitions,
                HARD_MAXIMUM_SMALL,
            ),
            ("maximum_surfaces", maximum_surfaces, HARD_MAXIMUM_SMALL),
            (
                "maximum_panel_instances",
                maximum_panel_instances,
                HARD_MAXIMUM_INSTANCES,
            ),
            (
                "maximum_panel_instances_per_region",
                maximum_panel_instances_per_region,
                HARD_MAXIMUM_INSTANCES,
            ),
        ];

        for (name, actual, maximum) in values {
            if actual == 0 {
                return Err(LayoutLimitsError::Zero { name });
            }
            if actual > maximum {
                return Err(LayoutLimitsError::ExceedsHardMaximum {
                    name,
                    maximum,
                    actual,
                });
            }
        }

        Ok(Self {
            maximum_schemas,
            maximum_regions_per_schema,
            maximum_sizing_slots_per_schema,
            maximum_panel_definitions,
            maximum_surfaces,
            maximum_panel_instances,
            maximum_panel_instances_per_region,
        })
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

    /// Returns the maximum Surfaces in one document.
    #[must_use]
    pub const fn maximum_surfaces(self) -> usize {
        self.maximum_surfaces
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

/// Invalid finite layout bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutLimitsError {
    /// A required limit was zero.
    Zero {
        /// Stable limit field name.
        name: &'static str,
    },
    /// A limit exceeded the library hard ceiling.
    ExceedsHardMaximum {
        /// Stable limit field name.
        name: &'static str,
        /// Library hard ceiling.
        maximum: usize,
        /// Rejected caller value.
        actual: usize,
    },
}

impl fmt::Display for LayoutLimitsError {
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

impl Error for LayoutLimitsError {}
