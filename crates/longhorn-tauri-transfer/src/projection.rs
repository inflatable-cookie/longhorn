use longhorn_core::{ClientPoint, ClientRect, ScreenPoint, ScreenRect, ScreenSize};

use crate::{ManagedTransferWindow, TransferProjectionError};

/// Projects one caller-local CSS point into the current global screen-DIP plane.
pub fn project_client_point(
    caller: &ManagedTransferWindow,
    point: ClientPoint,
) -> Result<ScreenPoint, TransferProjectionError> {
    let content = caller.content_bounds();
    let x = add_rounded(content.origin().x().get(), point.x().get(), f64::round)?;
    let y = add_rounded(content.origin().y().get(), point.y().get(), f64::round)?;
    let projected = ScreenPoint::new(x, y);
    if !content.contains_point(&projected) {
        return Err(TransferProjectionError::PointOutsideContent);
    }
    Ok(projected)
}

/// Projects one caller-local CSS rectangle using floor-left/top and ceil-right/bottom.
pub fn project_client_rect(
    caller: &ManagedTransferWindow,
    rect: ClientRect,
) -> Result<ScreenRect, TransferProjectionError> {
    let local_left = rect.origin().x().get();
    let local_top = rect.origin().y().get();
    let local_right = checked_add(local_left, rect.size().width().get())?;
    let local_bottom = checked_add(local_top, rect.size().height().get())?;
    let left = add_rounded(
        caller.content_bounds().origin().x().get(),
        local_left,
        f64::floor,
    )?;
    let top = add_rounded(
        caller.content_bounds().origin().y().get(),
        local_top,
        f64::floor,
    )?;
    let right = add_rounded(
        caller.content_bounds().origin().x().get(),
        local_right,
        f64::ceil,
    )?;
    let bottom = add_rounded(
        caller.content_bounds().origin().y().get(),
        local_bottom,
        f64::ceil,
    )?;
    let width = u32::try_from(
        right
            .checked_sub(left)
            .ok_or(TransferProjectionError::Overflow)?,
    )
    .map_err(|_| TransferProjectionError::Overflow)?;
    let height = u32::try_from(
        bottom
            .checked_sub(top)
            .ok_or(TransferProjectionError::Overflow)?,
    )
    .map_err(|_| TransferProjectionError::Overflow)?;
    if width == 0 || height == 0 {
        return Err(TransferProjectionError::EmptyRectangle);
    }
    let projected = ScreenRect::new(ScreenPoint::new(left, top), ScreenSize::new(width, height));
    if !caller.content_bounds().contains_rect(&projected) {
        return Err(TransferProjectionError::RectangleOutsideContent);
    }
    Ok(projected)
}

fn checked_add(left: f64, right: f64) -> Result<f64, TransferProjectionError> {
    let value = left + right;
    value
        .is_finite()
        .then_some(value)
        .ok_or(TransferProjectionError::NonFinite)
}

fn add_rounded(
    origin: i32,
    local: f64,
    round: fn(f64) -> f64,
) -> Result<i32, TransferProjectionError> {
    let rounded = round(local);
    if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err(TransferProjectionError::Overflow);
    }
    origin
        .checked_add(rounded as i32)
        .ok_or(TransferProjectionError::Overflow)
}
