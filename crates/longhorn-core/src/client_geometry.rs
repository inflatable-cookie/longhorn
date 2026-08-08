use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Finite window-content coordinate in logical (device-independent) pixels.
///
/// Host-neutral. A webview calls these CSS pixels and GPUI calls them
/// logical pixels; both mean the same thing — a device-independent unit
/// scaled to physical pixels by the display's scale factor.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct ClientLogicalPx(f64);

impl ClientLogicalPx {
    /// Validates and constructs a logical-pixel coordinate.
    pub fn new(value: f64) -> Result<Self, ClientGeometryError> {
        if !value.is_finite() {
            return Err(ClientGeometryError::NonFiniteValue);
        }

        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }

    /// Returns the finite logical-pixel value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for ClientLogicalPx {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.get())
    }
}

impl<'de> Deserialize<'de> for ClientLogicalPx {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// A window-content point in logical pixels.
///
/// Client-local coordinates cannot substitute for screen coordinates:
///
/// ```compile_fail
/// use longhorn_core::{ClientPoint, ScreenPoint};
///
/// let client = ClientPoint::new(10.0, 20.0).unwrap();
/// let _screen: ScreenPoint = client;
/// ```
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientPoint {
    x: ClientLogicalPx,
    y: ClientLogicalPx,
}

impl ClientPoint {
    /// Validates and constructs a point.
    pub fn new(x: f64, y: f64) -> Result<Self, ClientGeometryError> {
        Ok(Self {
            x: ClientLogicalPx::new(x)?,
            y: ClientLogicalPx::new(y)?,
        })
    }

    /// Returns the horizontal coordinate.
    #[must_use]
    pub const fn x(self) -> ClientLogicalPx {
        self.x
    }

    /// Returns the vertical coordinate.
    #[must_use]
    pub const fn y(self) -> ClientLogicalPx {
        self.y
    }
}

/// A non-negative window-content size in logical pixels.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(try_from = "UncheckedClientSize")]
pub struct ClientSize {
    width: ClientLogicalPx,
    height: ClientLogicalPx,
}

impl ClientSize {
    /// Validates and constructs a size.
    pub fn new(width: f64, height: f64) -> Result<Self, ClientGeometryError> {
        let width = ClientLogicalPx::new(width)?;
        let height = ClientLogicalPx::new(height)?;
        if width.get() < 0.0 || height.get() < 0.0 {
            return Err(ClientGeometryError::NegativeExtent);
        }

        Ok(Self { width, height })
    }

    /// Returns the width.
    #[must_use]
    pub const fn width(self) -> ClientLogicalPx {
        self.width
    }

    /// Returns the height.
    #[must_use]
    pub const fn height(self) -> ClientLogicalPx {
        self.height
    }
}

#[derive(Deserialize)]
struct UncheckedClientSize {
    width: f64,
    height: f64,
}

impl TryFrom<UncheckedClientSize> for ClientSize {
    type Error = ClientGeometryError;

    fn try_from(value: UncheckedClientSize) -> Result<Self, Self::Error> {
        Self::new(value.width, value.height)
    }
}

/// A window-content rectangle in logical pixels.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientRect {
    origin: ClientPoint,
    size: ClientSize,
}

impl ClientRect {
    /// Constructs a rectangle from validated client geometry.
    #[must_use]
    pub const fn new(origin: ClientPoint, size: ClientSize) -> Self {
        Self { origin, size }
    }

    /// Returns the origin.
    #[must_use]
    pub const fn origin(self) -> ClientPoint {
        self.origin
    }

    /// Returns the size.
    #[must_use]
    pub const fn size(self) -> ClientSize {
        self.size
    }
}

/// Client-local geometry validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientGeometryError {
    /// A client coordinate was NaN or infinite.
    NonFiniteValue,
    /// A client size contained a negative extent.
    NegativeExtent,
}

impl fmt::Display for ClientGeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => {
                formatter.write_str("client logical-pixel value must be finite")
            }
            Self::NegativeExtent => {
                formatter.write_str("client logical-pixel size cannot be negative")
            }
        }
    }
}

impl Error for ClientGeometryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_finite_coordinates_and_negative_sizes() {
        assert_eq!(
            ClientPoint::new(f64::NAN, 0.0),
            Err(ClientGeometryError::NonFiniteValue)
        );
        assert_eq!(
            ClientSize::new(-1.0, 2.0),
            Err(ClientGeometryError::NegativeExtent)
        );
        assert!(serde_json::from_str::<ClientSize>(r#"{"width":-1,"height":2}"#).is_err());
    }

    #[test]
    fn serde_normalizes_negative_zero() {
        let point = ClientPoint::new(-0.0, 1.25).unwrap();

        assert_eq!(
            serde_json::to_string(&point).unwrap(),
            r#"{"x":0.0,"y":1.25}"#
        );
    }
}
