use longhorn_core::ScaleFactor;

use crate::TauriScaleFactorError;

/// Converts a positive finite Tauri scale to fixed-point thousandths.
///
/// Ties use `f64::round`, which is away from zero. Scale is positive, so this
/// matches Longhorn's explicit nearest rounding at the host edge.
pub fn scale_factor_from_tauri(value: f64) -> Result<ScaleFactor, TauriScaleFactorError> {
    if !value.is_finite() {
        return Err(TauriScaleFactorError::NonFinite);
    }
    if value <= 0.0 {
        return Err(TauriScaleFactorError::NonPositive);
    }

    let rounded = (value * 1000.0).round();
    if rounded < 1.0 {
        return Err(TauriScaleFactorError::RoundedToZero);
    }
    if rounded > f64::from(u32::MAX) {
        return Err(TauriScaleFactorError::Overflow);
    }

    // Range checks above make the float-to-integer cast exact in domain.
    ScaleFactor::from_thousandths(rounded as u32).map_err(|_| TauriScaleFactorError::RoundedToZero)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_rounds_to_nearest_thousandth() {
        assert_eq!(scale_factor_from_tauri(1.2504).unwrap().thousandths(), 1250);
        assert_eq!(scale_factor_from_tauri(1.2505).unwrap().thousandths(), 1251);
        assert_eq!(
            scale_factor_from_tauri(f64::NAN),
            Err(TauriScaleFactorError::NonFinite)
        );
        assert_eq!(
            scale_factor_from_tauri(f64::INFINITY),
            Err(TauriScaleFactorError::NonFinite)
        );
        assert_eq!(
            scale_factor_from_tauri(0.0),
            Err(TauriScaleFactorError::NonPositive)
        );
        assert_eq!(
            scale_factor_from_tauri(0.000_1),
            Err(TauriScaleFactorError::RoundedToZero)
        );
        assert_eq!(
            scale_factor_from_tauri(f64::from(u32::MAX) / 1000.0 + 1.0),
            Err(TauriScaleFactorError::Overflow)
        );
    }
}
