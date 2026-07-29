use super::*;

fn key(x: i32, scale: f64) -> MonitorMatchKey {
    MonitorMatchKey {
        name: Some("Display".to_string()),
        position: (x, 0),
        size: (1920, 1080),
        work_position: (x, 24),
        work_size: (1920, 1056),
        scale_bits: scale.to_bits(),
    }
}

#[test]
fn primary_key_keeps_raw_scale_bits_exact() {
    let primary = key(0, 1.250_4);
    assert_ne!(primary, key(0, 1.250_49));
    assert_eq!(primary, key(0, 1.250_4));
    assert_eq!(
        exact_primary_key_index(&[key(-1920, 1.0), primary.clone()], &primary),
        Ok(1)
    );
    assert_eq!(
        exact_primary_key_index(&[primary.clone(), primary.clone()], &primary),
        Err(TauriProbeError::AmbiguousPrimaryMonitor { matches: 2 })
    );
    assert_eq!(
        exact_primary_key_index(&[key(1920, 1.0)], &primary),
        Err(TauriProbeError::PrimaryMonitorNotFound)
    );
}
