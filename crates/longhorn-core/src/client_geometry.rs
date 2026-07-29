use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// Finite local webview content coordinate measured in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct ClientCssPx(f64);

impl ClientCssPx {
    /// Validates and constructs a client CSS coordinate.
    pub fn new(value: f64) -> Result<Self, ClientGeometryError> {
        if !value.is_finite() {
            return Err(ClientGeometryError::NonFiniteValue);
        }

        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }

    /// Returns the finite CSS-pixel value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for ClientCssPx {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.get())
    }
}

impl<'de> Deserialize<'de> for ClientCssPx {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// A local webview content point.
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
    x: ClientCssPx,
    y: ClientCssPx,
}

impl ClientPoint {
    /// Validates and constructs a point.
    pub fn new(x: f64, y: f64) -> Result<Self, ClientGeometryError> {
        Ok(Self {
            x: ClientCssPx::new(x)?,
            y: ClientCssPx::new(y)?,
        })
    }

    /// Returns the horizontal coordinate.
    #[must_use]
    pub const fn x(self) -> ClientCssPx {
        self.x
    }

    /// Returns the vertical coordinate.
    #[must_use]
    pub const fn y(self) -> ClientCssPx {
        self.y
    }
}

/// A non-negative local webview content size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(try_from = "UncheckedClientSize")]
pub struct ClientSize {
    width: ClientCssPx,
    height: ClientCssPx,
}

impl ClientSize {
    /// Validates and constructs a size.
    pub fn new(width: f64, height: f64) -> Result<Self, ClientGeometryError> {
        let width = ClientCssPx::new(width)?;
        let height = ClientCssPx::new(height)?;
        if width.get() < 0.0 || height.get() < 0.0 {
            return Err(ClientGeometryError::NegativeExtent);
        }

        Ok(Self { width, height })
    }

    /// Returns the width.
    #[must_use]
    pub const fn width(self) -> ClientCssPx {
        self.width
    }

    /// Returns the height.
    #[must_use]
    pub const fn height(self) -> ClientCssPx {
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

/// A local webview content rectangle.
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
            Self::NonFiniteValue => formatter.write_str("client CSS value must be finite"),
            Self::NegativeExtent => formatter.write_str("client CSS size cannot be negative"),
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
