use longhorn_core::{
    ClientRect, PhysicalPoint, PhysicalRect, PhysicalSize, RoundingMode, ScaleFactor,
};

use crate::ViewportConversionError;

/// Converts a semantic client viewport to checked physical geometry.
pub fn viewport_to_physical(
    viewport: ClientRect,
    scale: ScaleFactor,
    rounding: RoundingMode,
) -> Result<PhysicalRect, ViewportConversionError> {
    let multiplier = f64::from(scale.thousandths()) / 1000.0;
    let origin = viewport.origin();
    let size = viewport.size();
    let x = round(origin.x().get() * multiplier, rounding);
    let y = round(origin.y().get() * multiplier, rounding);
    let width = round(size.width().get() * multiplier, rounding);
    let height = round(size.height().get() * multiplier, rounding);

    let x = to_i32(x)?;
    let y = to_i32(y)?;
    let width = to_u32(width)?;
    let height = to_u32(height)?;

    Ok(PhysicalRect::new(
        PhysicalPoint::new(x, y),
        PhysicalSize::new(width, height),
    ))
}

fn round(value: f64, mode: RoundingMode) -> f64 {
    match mode {
        RoundingMode::Floor => value.floor(),
        RoundingMode::Ceil => value.ceil(),
        RoundingMode::Nearest => value.round(),
    }
}

fn to_i32(value: f64) -> Result<i32, ViewportConversionError> {
    if value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
        return Err(ViewportConversionError::CoordinateOverflow);
    }
    Ok(value as i32)
}

fn to_u32(value: f64) -> Result<u32, ViewportConversionError> {
    if value < 0.0 || value > f64::from(u32::MAX) {
        return Err(ViewportConversionError::ExtentOverflow);
    }
    Ok(value as u32)
}
