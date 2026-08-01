use std::{error::Error, fmt, num::NonZeroU32};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    PhysicalPoint, PhysicalPx, PhysicalRect, PhysicalSize, Point, Rect, ScreenDip, ScreenPoint,
    ScreenRect, ScreenSize, Size,
};

const SCALE_BASE: i128 = 1000;

/// Explicit rounding policy for scale conversion.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RoundingMode {
    /// Round toward negative infinity.
    Floor,
    /// Round toward positive infinity.
    Ceil,
    /// Round to the nearest integer, with exact ties away from zero.
    Nearest,
}

/// Positive fixed-point scale evidence in thousandths.
///
/// `1000` represents an exact scale of `1.0`.
/// Nearest integer physical-to-screen-to-physical conversion has a maximum
/// error of `ceil(thousandths / 2000)` physical pixels.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
pub struct ScaleFactor(NonZeroU32);

impl ScaleFactor {
    /// Validates and constructs scale evidence from integer thousandths.
    pub const fn from_thousandths(value: u32) -> Result<Self, ScaleFactorError> {
        match NonZeroU32::new(value) {
            Some(value) => Ok(Self(value)),
            None => Err(ScaleFactorError::Zero),
        }
    }

    /// Returns the serialized integer thousandths.
    #[must_use]
    pub const fn thousandths(self) -> u32 {
        self.0.get()
    }

    /// Converts a physical coordinate to global screen DIPs.
    pub fn physical_to_screen(
        self,
        value: PhysicalPx,
        rounding: RoundingMode,
    ) -> Result<ScreenDip, ScaleConversionError> {
        let value = round_ratio(
            i128::from(value.get()) * SCALE_BASE,
            i128::from(self.thousandths()),
            rounding,
        );
        Ok(ScreenDip::new(checked_i32(value)?))
    }

    /// Converts a global screen-DIP coordinate to physical pixels.
    pub fn screen_to_physical(
        self,
        value: ScreenDip,
        rounding: RoundingMode,
    ) -> Result<PhysicalPx, ScaleConversionError> {
        let value = round_ratio(
            i128::from(value.get()) * i128::from(self.thousandths()),
            SCALE_BASE,
            rounding,
        );
        Ok(PhysicalPx::new(checked_i32(value)?))
    }

    /// Converts a physical point to global screen DIPs.
    pub fn physical_point_to_screen(
        self,
        point: PhysicalPoint,
        rounding: RoundingMode,
    ) -> Result<ScreenPoint, ScaleConversionError> {
        Ok(Point::from_coordinates(
            self.physical_to_screen(point.x(), rounding)?,
            self.physical_to_screen(point.y(), rounding)?,
        ))
    }

    /// Converts a global screen-DIP point to physical pixels.
    pub fn screen_point_to_physical(
        self,
        point: ScreenPoint,
        rounding: RoundingMode,
    ) -> Result<PhysicalPoint, ScaleConversionError> {
        Ok(Point::from_coordinates(
            self.screen_to_physical(point.x(), rounding)?,
            self.screen_to_physical(point.y(), rounding)?,
        ))
    }

    /// Converts a physical size to global screen DIPs.
    pub fn physical_size_to_screen(
        self,
        size: PhysicalSize,
        rounding: RoundingMode,
    ) -> Result<ScreenSize, ScaleConversionError> {
        Ok(Size::new(
            physical_extent_to_screen(size.width(), self, rounding)?,
            physical_extent_to_screen(size.height(), self, rounding)?,
        ))
    }

    /// Converts a global screen-DIP size to physical pixels.
    pub fn screen_size_to_physical(
        self,
        size: ScreenSize,
        rounding: RoundingMode,
    ) -> Result<PhysicalSize, ScaleConversionError> {
        Ok(Size::new(
            screen_extent_to_physical(size.width(), self, rounding)?,
            screen_extent_to_physical(size.height(), self, rounding)?,
        ))
    }

    /// Converts a physical rectangle by converting origin and size separately.
    pub fn physical_rect_to_screen(
        self,
        rect: PhysicalRect,
        rounding: RoundingMode,
    ) -> Result<ScreenRect, ScaleConversionError> {
        Ok(Rect::new(
            self.physical_point_to_screen(rect.origin(), rounding)?,
            self.physical_size_to_screen(rect.size(), rounding)?,
        ))
    }

    /// Converts a screen-DIP rectangle by converting origin and size separately.
    pub fn screen_rect_to_physical(
        self,
        rect: ScreenRect,
        rounding: RoundingMode,
    ) -> Result<PhysicalRect, ScaleConversionError> {
        Ok(Rect::new(
            self.screen_point_to_physical(rect.origin(), rounding)?,
            self.screen_size_to_physical(rect.size(), rounding)?,
        ))
    }
}

impl TryFrom<u32> for ScaleFactor {
    type Error = ScaleFactorError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_thousandths(value)
    }
}

impl From<ScaleFactor> for u32 {
    fn from(value: ScaleFactor) -> Self {
        value.thousandths()
    }
}

impl Serialize for ScaleFactor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.thousandths())
    }
}

impl<'de> Deserialize<'de> for ScaleFactor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Self::from_thousandths(value).map_err(de::Error::custom)
    }
}

fn physical_extent_to_screen(
    value: u32,
    scale: ScaleFactor,
    rounding: RoundingMode,
) -> Result<u32, ScaleConversionError> {
    checked_u32(round_ratio(
        i128::from(value) * SCALE_BASE,
        i128::from(scale.thousandths()),
        rounding,
    ))
}

fn screen_extent_to_physical(
    value: u32,
    scale: ScaleFactor,
    rounding: RoundingMode,
) -> Result<u32, ScaleConversionError> {
    checked_u32(round_ratio(
        i128::from(value) * i128::from(scale.thousandths()),
        SCALE_BASE,
        rounding,
    ))
}

fn round_ratio(numerator: i128, denominator: i128, rounding: RoundingMode) -> i128 {
    match rounding {
        RoundingMode::Floor => numerator.div_euclid(denominator),
        RoundingMode::Ceil => {
            let quotient = numerator.div_euclid(denominator);
            if numerator.rem_euclid(denominator) == 0 {
                quotient
            } else {
                quotient + 1
            }
        }
        RoundingMode::Nearest => {
            let sign = if numerator < 0 { -1 } else { 1 };
            let absolute = numerator.abs();
            let quotient = absolute / denominator;
            let remainder = absolute % denominator;
            sign * if remainder * 2 >= denominator {
                quotient + 1
            } else {
                quotient
            }
        }
    }
}

fn checked_i32(value: i128) -> Result<i32, ScaleConversionError> {
    i32::try_from(value).map_err(|_| ScaleConversionError::Overflow)
}

fn checked_u32(value: i128) -> Result<u32, ScaleConversionError> {
    u32::try_from(value).map_err(|_| ScaleConversionError::Overflow)
}

/// Scale-factor validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScaleFactorError {
    /// Scale zero is invalid.
    Zero,
}

impl fmt::Display for ScaleFactorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("scale factor must be positive")
    }
}

impl Error for ScaleFactorError {}

/// Checked physical/logical conversion failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScaleConversionError {
    /// The converted value could not be represented by the target type.
    Overflow,
}

impl fmt::Display for ScaleConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("scale conversion overflowed the target type")
    }
}

impl Error for ScaleConversionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_is_explicit_for_negative_values() {
        let scale = ScaleFactor::from_thousandths(1500).unwrap();
        let value = PhysicalPx::new(-2);

        assert_eq!(
            scale
                .physical_to_screen(value, RoundingMode::Floor)
                .unwrap()
                .get(),
            -2
        );
        assert_eq!(
            scale
                .physical_to_screen(value, RoundingMode::Ceil)
                .unwrap()
                .get(),
            -1
        );
        assert_eq!(
            scale
                .physical_to_screen(value, RoundingMode::Nearest)
                .unwrap()
                .get(),
            -1
        );
    }

    #[test]
    fn zero_is_rejected_by_constructor_and_serde() {
        assert_eq!(
            ScaleFactor::from_thousandths(0),
            Err(ScaleFactorError::Zero)
        );
        assert!(serde_json::from_str::<ScaleFactor>("0").is_err());
    }
}
