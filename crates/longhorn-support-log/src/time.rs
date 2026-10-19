//! Std-only UTC timestamps.

use std::time::{SystemTime, UNIX_EPOCH};

const MILLIS_PER_DAY: i128 = 86_400_000;

/// Formats `at` in UTC as RFC 3339 with milliseconds and `Z`:
/// `2026-10-01T12:00:00.000Z`. Sub-millisecond time is truncated, never
/// rounded, so a carry never moves the second.
#[must_use]
pub fn utc_timestamp(at: SystemTime) -> String {
    let millis = match at.duration_since(UNIX_EPOCH) {
        Ok(after) => i128::try_from(after.as_millis()).unwrap_or(i128::MAX),
        // Truncate toward the past so `.999` stays in its own second.
        Err(before) => {
            let before = before.duration();
            let whole = i128::try_from(before.as_millis()).unwrap_or(i128::MAX);
            if before.subsec_nanos() % 1_000_000 == 0 {
                -whole
            } else {
                -whole - 1
            }
        }
    };
    let days = millis.div_euclid(MILLIS_PER_DAY);
    let of_day = millis.rem_euclid(MILLIS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        of_day / 3_600_000,
        of_day / 60_000 % 60,
        of_day / 1_000 % 60,
        of_day % 1_000,
    )
}

/// Proleptic Gregorian date for a day count from 1970-01-01, after Howard
/// Hinnant's `civil_from_days`.
fn civil_from_days(days: i128) -> (i128, i128, i128) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i128::from(month <= 2);
    (year, month, day)
}
