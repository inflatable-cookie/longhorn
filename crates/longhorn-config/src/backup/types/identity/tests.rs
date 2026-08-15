//! Unit and property tests for backup identity metadata. The
//! `parse_utc_timestamp` properties (card 213) run 64 fixed cases each;
//! measured cost is a few milliseconds for the whole module.

use proptest::prelude::*;

use super::*;

#[test]
fn limits_reject_zero_inversion_and_hard_ceiling() {
    assert_eq!(BackupLimits::new(0, 1), Err(BackupLimitsError::Zero));
    assert_eq!(
        BackupLimits::new(2, 1),
        Err(BackupLimitsError::DomainExceedsTotal)
    );
    assert!(matches!(
        BackupLimits::new(HARD_MAX_DOMAIN_BYTES + 1, HARD_MAX_TOTAL_BYTES + 1),
        Err(BackupLimitsError::HardCeiling { .. })
    ));
}

#[test]
fn utc_timestamp_is_strict_and_orders_fractional_seconds() {
    let epoch = parse_utc_timestamp("1970-01-01T00:00:00Z").unwrap();
    assert_eq!(epoch.seconds, 0);
    assert_eq!(epoch.nanoseconds, 0);
    let leap = parse_utc_timestamp("2024-02-29T23:59:59.25Z").unwrap();
    let later = parse_utc_timestamp("2024-02-29T23:59:59.250000001Z").unwrap();
    assert!(leap < later);
    assert!(parse_utc_timestamp("2023-02-29T00:00:00Z").is_err());
    assert!(parse_utc_timestamp("2026-01-01T00:00:00+00:00").is_err());
    assert!(parse_utc_timestamp("2026-01-01T00:00:60Z").is_err());
}

/// Reference leap-year rule, independent of the parser's formulation.
const fn leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

const fn reference_days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year(year) => 29,
        _ => 28,
    }
}

/// Days since 1970-01-01 by plain year and month iteration, independent of
/// the parser's `days_from_civil` arithmetic.
fn reference_unix_seconds(
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> i64 {
    let mut days = 0_i64;
    if year >= 1970 {
        for y in 1970..year {
            days += if leap_year(y) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            days -= if leap_year(y) { 366 } else { 365 };
        }
    }
    for m in 1..month {
        days += i64::from(reference_days_in_month(year, m));
    }
    days += i64::from(day - 1);
    days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second)
}

fn timestamp_strategy() -> impl Strategy<Value = (u32, u32, u32, u32, u32, u32, Vec<u8>)> {
    (1..=9_999_u32, 1..=12_u32).prop_flat_map(|(year, month)| {
        (
            Just(year),
            Just(month),
            1..=reference_days_in_month(year, month),
            0..=23_u32,
            0..=59_u32,
            0..=59_u32,
            prop::collection::vec(b'0'..=b'9', 1..=9),
        )
    })
}

fn format_timestamp(
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    fraction: &[u8],
) -> String {
    let mut value = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}");
    if !fraction.is_empty() {
        value.push('.');
        value.push_str(std::str::from_utf8(fraction).expect("digit fraction"));
    }
    value.push('Z');
    value
}

/// Strings biased toward the grammar alphabet, so generated cases cluster on
/// the accepted shape's edges rather than being uniformly non-timestamps.
fn timestampish_string() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            4 => b'0'..=b'9',
            1 => prop_oneof![
                Just(b'-'),
                Just(b'T'),
                Just(b':'),
                Just(b'.'),
                Just(b'Z'),
                Just(b'+'),
                Just(b' '),
                Just(b'e'),
                Just(0xff_u8),
            ],
        ],
        0..=40,
    )
    .prop_map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// The documented grammar shape, restated: accepted input must look exactly
/// like `YYYY-MM-DDTHH:MM:SS[.fraction]Z` and nothing else.
fn matches_documented_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    matches!(bytes.len(), 20 | 22..=30)
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && bytes.last() == Some(&b'Z')
        && (bytes.len() == 20 || bytes.get(19) == Some(&b'.'))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Every generated calendar-valid timestamp parses, lands on the epoch
    /// second an independent reference computes, and carries the fractional
    /// digits scaled to nanoseconds. Padding the fraction to nine digits, or
    /// dropping an all-zero fraction, parses to the same instant.
    #[test]
    fn generated_valid_timestamps_round_trip(
        (year, month, day, hour, minute, second, fraction) in timestamp_strategy(),
    ) {
        let value = format_timestamp(year, month, day, hour, minute, second, &fraction);
        let parsed = parse_utc_timestamp(&value).expect("generated timestamp must parse");

        prop_assert_eq!(
            parsed.seconds,
            reference_unix_seconds(year, month, day, hour, minute, second),
        );
        let mut expected_nanos = 0_u32;
        for digit in &fraction {
            expected_nanos = expected_nanos * 10 + u32::from(digit - b'0');
        }
        expected_nanos *= 10_u32.pow(9 - fraction.len() as u32);
        prop_assert_eq!(parsed.nanoseconds, expected_nanos);

        let padded = format_timestamp(
            year,
            month,
            day,
            hour,
            minute,
            second,
            &format!("{:0<9}", std::str::from_utf8(&fraction).unwrap())
                .into_bytes(),
        );
        prop_assert_eq!(parse_utc_timestamp(&padded).unwrap(), parsed);
        if fraction.iter().all(|digit| *digit == b'0') {
            let plain = format_timestamp(year, month, day, hour, minute, second, &[]);
            prop_assert_eq!(parse_utc_timestamp(&plain).unwrap(), parsed);
        }
    }

    /// Arbitrary grammar-adjacent strings never panic the parser, never
    /// silently reinterpret a non-canonical shape (accepted input matches the
    /// documented grammar exactly), and parsing is idempotent.
    #[test]
    fn arbitrary_strings_accept_only_the_documented_shape(value in timestampish_string()) {
        if let Ok(parsed) = parse_utc_timestamp(&value) {
            prop_assert!(
                matches_documented_shape(&value),
                "accepted non-canonical input: {value:?}"
            );
            prop_assert_eq!(parse_utc_timestamp(&value).unwrap(), parsed);
        }
    }
}
