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
