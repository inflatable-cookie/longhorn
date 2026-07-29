use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, de};

const HARD_MAXIMUM_RECORDS: usize = 4_096;
const HARD_MAXIMUM_LABEL_BYTES: usize = 16_384;

/// Explicit finite bounds for one Surface document and resolution input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SurfaceLimits {
    maximum_surfaces: usize,
    maximum_windows: usize,
    maximum_host_preferences_per_surface: usize,
    maximum_label_bytes: usize,
}

impl<'de> Deserialize<'de> for SurfaceLimits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SerializedLimits {
            maximum_surfaces: usize,
            maximum_windows: usize,
            maximum_host_preferences_per_surface: usize,
            maximum_label_bytes: usize,
        }

        let limits = SerializedLimits::deserialize(deserializer)?;
        Self::new(
            limits.maximum_surfaces,
            limits.maximum_windows,
            limits.maximum_host_preferences_per_surface,
            limits.maximum_label_bytes,
        )
        .map_err(de::Error::custom)
    }
}

impl SurfaceLimits {
    /// Constructs limits and rejects zero or excessive values.
    pub fn new(
        maximum_surfaces: usize,
        maximum_windows: usize,
        maximum_host_preferences_per_surface: usize,
        maximum_label_bytes: usize,
    ) -> Result<Self, SurfaceLimitsError> {
        let values = [
            ("maximum_surfaces", maximum_surfaces, HARD_MAXIMUM_RECORDS),
            ("maximum_windows", maximum_windows, HARD_MAXIMUM_RECORDS),
            (
                "maximum_host_preferences_per_surface",
                maximum_host_preferences_per_surface,
                HARD_MAXIMUM_RECORDS,
            ),
            (
                "maximum_label_bytes",
                maximum_label_bytes,
                HARD_MAXIMUM_LABEL_BYTES,
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

        Ok(Self {
            maximum_surfaces,
            maximum_windows,
            maximum_host_preferences_per_surface,
            maximum_label_bytes,
        })
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
