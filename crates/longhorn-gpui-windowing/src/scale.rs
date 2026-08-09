use longhorn_core::ScaleFactor;

use crate::GpuiScaleFactorError;

/// Converts a positive finite GPUI scale to fixed-point thousandths.
///
/// GPUI reports scale as `f32`, not `f64`. The widening is exact, so the
/// rounding policy is the same explicit nearest rounding the Tauri edge uses
/// and the two hosts agree on every representable scale.
pub fn scale_factor_from_gpui(value: f32) -> Result<ScaleFactor, GpuiScaleFactorError> {
    if !value.is_finite() {
        return Err(GpuiScaleFactorError::NonFinite);
    }
    if value <= 0.0 {
        return Err(GpuiScaleFactorError::NonPositive);
    }

    let rounded = (f64::from(value) * 1000.0).round();
    if rounded < 1.0 {
        return Err(GpuiScaleFactorError::RoundedToZero);
    }
    if rounded > f64::from(u32::MAX) {
        return Err(GpuiScaleFactorError::Overflow);
    }

    // Range checks above make the float-to-integer cast exact in domain.
    ScaleFactor::from_thousandths(rounded as u32).map_err(|_| GpuiScaleFactorError::RoundedToZero)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_rounds_to_nearest_thousandth() {
        assert_eq!(scale_factor_from_gpui(2.0).unwrap().thousandths(), 2000);
        assert_eq!(scale_factor_from_gpui(1.5).unwrap().thousandths(), 1500);
        assert_eq!(
            scale_factor_from_gpui(f32::NAN),
            Err(GpuiScaleFactorError::NonFinite)
        );
        assert_eq!(
            scale_factor_from_gpui(f32::INFINITY),
            Err(GpuiScaleFactorError::NonFinite)
        );
        assert_eq!(
            scale_factor_from_gpui(0.0),
            Err(GpuiScaleFactorError::NonPositive)
        );
        assert_eq!(
            scale_factor_from_gpui(-2.0),
            Err(GpuiScaleFactorError::NonPositive)
        );
        assert_eq!(
            scale_factor_from_gpui(0.000_1),
            Err(GpuiScaleFactorError::RoundedToZero)
        );
    }

    #[test]
    fn the_two_hosts_agree_on_every_scale_a_display_actually_reports() {
        // macOS reports 1.0 and 2.0; Windows and Linux add fractional steps.
        // Tauri hands these over as f64 and GPUI as f32, so the adapters would
        // disagree if either widened or rounded differently.
        for scale in [1.0_f32, 1.25, 1.5, 1.75, 2.0, 3.0] {
            assert_eq!(
                scale_factor_from_gpui(scale).unwrap().thousandths(),
                (f64::from(scale) * 1000.0).round() as u32
            );
        }
    }
}
