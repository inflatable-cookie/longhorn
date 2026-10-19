//! Contract 025 through the public API only.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use longhorn_redact::{RedactionRules, Redactor};
use longhorn_support_log::{
    JsonRecord, Level, LogStats, OMITTED_MARKER, RingLog, RotatingLog, RotatingLogConfig,
    TailBounds, bound_field, bounded_tail, utc_timestamp,
};

const FILE: &str = "support.log";
const MAX_BYTES: u64 = 512 * 1024;
const FIELD_BYTES: usize = 48 * 1024;
const TAIL: TailBounds = TailBounds {
    lines: 200,
    bytes: 64 * 1024,
};

fn redactor() -> Redactor {
    Redactor::new(Vec::new(), RedactionRules::common())
}

fn config(directory: &Path, max_bytes: u64) -> RotatingLogConfig {
    RotatingLogConfig {
        directory: directory.to_path_buf(),
        file_name: FILE.to_owned(),
        max_bytes,
        retained: 2,
        field_bytes: FIELD_BYTES,
    }
}

fn at(seconds: i64, nanos: u32) -> SystemTime {
    let base = if seconds >= 0 {
        UNIX_EPOCH + Duration::from_secs(seconds.unsigned_abs())
    } else {
        UNIX_EPOCH - Duration::from_secs(seconds.unsigned_abs())
    };
    base + Duration::from_nanos(u64::from(nanos))
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

fn generations(root: &Path) -> [std::path::PathBuf; 3] {
    [
        root.join(FILE),
        root.join(format!("{FILE}.1")),
        root.join(format!("{FILE}.2")),
    ]
}

const PEM_BODY: [&str; 4] = [
    "MIIEvQIBADANBgkqhkiG9w0BAQEFAASC",
    "BKcwggSjAgEAAoIBAQC7VJTUt9Us8cKj",
    "MzEfYyjiWA4R4/M2bS1GB4t7NXp98C3S",
    "C2qkUs6NqvrlLz2Md0lKzq7yTCLSWZfR",
];

fn pem_block() -> String {
    format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----",
        PEM_BODY.join("\n")
    )
}

// --- timestamps -----------------------------------------------------------

#[test]
fn timestamps_are_utc_rfc3339_with_truncated_milliseconds() {
    let cases = [
        (at(0, 0), "1970-01-01T00:00:00.000Z"),
        (at(1_790_856_000, 0), "2026-10-01T12:00:00.000Z"),
        // Leap day, last millisecond.
        (at(1_709_251_199, 999_000_000), "2024-02-29T23:59:59.999Z"),
        (at(1_709_251_200, 0), "2024-03-01T00:00:00.000Z"),
        // Sub-millisecond time truncates; it never carries into the second.
        (at(946_684_799, 999_999_999), "1999-12-31T23:59:59.999Z"),
        // One more nanosecond carries through every field.
        (at(946_684_800, 0), "2000-01-01T00:00:00.000Z"),
        // 2100 is not a leap year.
        (at(4_107_542_400 - 1, 0), "2100-02-28T23:59:59.000Z"),
        (at(4_107_542_400, 0), "2100-03-01T00:00:00.000Z"),
        (at(-1, 999_000_000), "1969-12-31T23:59:59.999Z"),
        (at(-1, 999_999_999), "1969-12-31T23:59:59.999Z"),
        (at(-2_203_891_200, 0), "1900-03-01T00:00:00.000Z"),
    ];
    for (instant, expected) in cases {
        assert_eq!(utc_timestamp(instant), expected);
    }
}

// --- ring log -------------------------------------------------------------

#[test]
fn ring_keeps_its_capacity_and_tails_oldest_first() {
    let log = RingLog::new(400, FIELD_BYTES, redactor());
    assert!(log.is_empty());
    for index in 0..450 {
        log.record(Level::Info, "scope", &format!("message {index}"));
    }
    assert_eq!(log.len(), 400);

    let tail = log.tail(100);
    assert_eq!(tail.len(), 100);
    for (offset, record) in tail.iter().enumerate() {
        assert_eq!(record.message(), format!("message {}", 350 + offset));
    }
    assert_eq!(log.tail(1_000).len(), 400);
    assert_eq!(
        log.tail(1_000)
            .first()
            .map(|record| record.message().to_owned()),
        Some("message 50".to_owned())
    );
}

#[test]
fn ring_lines_are_stable_redacted_and_single_line() {
    let log = RingLog::new(4, 18, redactor());
    log.record_at(at(1_790_856_000, 0), Level::Error, "github", "detail");
    log.record_at(
        at(1_790_856_000, 0),
        Level::Warning,
        "push",
        "token=ghp_abcdef\nnext",
    );
    let lines = log
        .tail(4)
        .iter()
        .map(|record| record.line())
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        [
            "2026-10-01T12:00:00.000Z [error] github: detail",
            "2026-10-01T12:00:00.000Z [warning] push: token=[redacted]\\nn",
        ]
    );
    assert_eq!(log.tail(4)[1].level(), Level::Warning);
}

#[test]
fn a_zero_capacity_ring_holds_nothing() {
    let log = RingLog::new(0, FIELD_BYTES, redactor());
    log.record(Level::Error, "scope", "message");
    assert!(log.is_empty());
}

// --- rotating log: donor tests -------------------------------------------

#[test]
fn records_to_the_configured_directory_and_rotates_at_the_bound() {
    let root = tempfile::tempdir().unwrap();
    let log = RotatingLog::open(config(root.path(), MAX_BYTES), redactor());

    log.record_at(at(1_790_856_000, 0), "apply.start", "version=1.1.0");
    let contents = fs::read_to_string(root.path().join(FILE)).unwrap();
    assert_eq!(
        contents,
        "2026-10-01T12:00:00.000Z apply.start version=1.1.0\n"
    );

    // A live file at the bound rotates before the next line.
    fs::write(root.path().join(FILE), vec![b'x'; MAX_BYTES as usize]).unwrap();
    log.record("apply.ok", "relaunched=false");
    assert_eq!(file_len(&root.path().join(format!("{FILE}.1"))), MAX_BYTES);
    let fresh = fs::read_to_string(root.path().join(FILE)).unwrap();
    assert!(fresh.ends_with(" apply.ok relaunched=false\n"), "{fresh}");
    assert_eq!(log.stats().written, 2);
}

#[test]
fn a_retained_handle_still_enforces_the_bound_on_every_write() {
    let root = tempfile::tempdir().unwrap();
    let log = RotatingLog::open(config(root.path(), MAX_BYTES), redactor());
    let detail = "x".repeat(1024);
    for _ in 0..800 {
        log.record("apply.start", &detail);
    }

    for path in generations(root.path()) {
        assert!(path.exists(), "{}", path.display());
        assert!(file_len(&path) <= MAX_BYTES, "{}", path.display());
    }
    assert!(!root.path().join(format!("{FILE}.3")).exists());
    assert_eq!(
        log.stats(),
        LogStats {
            written: 800,
            oversized: 0,
            failed: 0
        }
    );
}

#[test]
fn a_discarding_log_never_opens_a_file() {
    let log = RotatingLog::discarding();
    log.record("apply.start", "version=1.1.0");
    log.record_json(&JsonRecord::new().text("event", "failed"));
    assert_eq!(log.directory(), None);
    assert_eq!(log.stats(), LogStats::default());
}

#[test]
fn json_records_redact_each_value_before_serializing() {
    let root = tempfile::tempdir().unwrap();
    let log = RotatingLog::open(config(root.path(), MAX_BYTES), redactor());
    let record = JsonRecord::new()
        .text("timestamp", &utc_timestamp(at(1_790_856_000, 0)))
        .text("tool", "claude")
        .text("event", "failed")
        .optional_text("outcome", None)
        .text(
            "detail",
            "token=private-value authorization: Bearer private-bearer",
        )
        // A quote inside a value cannot join a redaction span to the
        // serialized line around it.
        .text("summary", "said \"password=hunter22\" twice")
        .optional_integer("exitStatus", Some(1))
        .integer("attempt", -3);
    log.record_json(&record);

    let contents = fs::read_to_string(root.path().join(FILE)).unwrap();
    assert_eq!(
        contents,
        concat!(
            r#"{"timestamp":"2026-10-01T12:00:00.000Z","tool":"claude","event":"failed","#,
            r#""detail":"token=[redacted] authorization: Bearer [redacted]","#,
            r#""summary":"said \"password=[redacted] twice","exitStatus":1,"attempt":-3}"#,
            "\n"
        )
    );
}

// --- rotating log: contract rules ----------------------------------------

#[test]
fn two_threads_never_interleave_and_keep_the_bound() {
    let root = tempfile::tempdir().unwrap();
    let max_bytes = 8 * 1024;
    let log = Arc::new(RotatingLog::open(
        config(root.path(), max_bytes),
        redactor(),
    ));
    let workers = ["left", "right"].map(|name| {
        let log = Arc::clone(&log);
        thread::spawn(move || {
            for index in 0..2_000 {
                log.record(name, &format!("{index:04}-{}", "y".repeat(64)));
            }
        })
    });
    for worker in workers {
        worker.join().unwrap();
    }

    assert_eq!(log.stats().written, 4_000);
    for path in generations(root.path()) {
        assert!(file_len(&path) <= max_bytes, "{}", path.display());
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.ends_with('\n'));
        for line in contents.lines() {
            let fields = line.split(' ').collect::<Vec<_>>();
            assert_eq!(fields.len(), 3, "{line}");
            assert_eq!(fields[0].len(), "2026-10-01T12:00:00.000Z".len(), "{line}");
            assert!(matches!(fields[1], "left" | "right"), "{line}");
            assert_eq!(fields[2].len(), 5 + 64, "{line}");
        }
    }
}

#[test]
fn a_restart_that_finds_a_full_live_file_rotates_before_writing() {
    let root = tempfile::tempdir().unwrap();
    let max_bytes = 1024;
    let live = root.path().join(FILE);
    fs::write(&live, vec![b'x'; 1000]).unwrap();
    fs::write(root.path().join(format!("{FILE}.1")), b"older\n").unwrap();
    fs::write(root.path().join(format!("{FILE}.2")), b"oldest\n").unwrap();

    // A new process opens a fresh log over the existing files.
    let log = RotatingLog::open(config(root.path(), max_bytes), redactor());
    log.record("start", &"z".repeat(64));

    assert_eq!(file_len(&root.path().join(format!("{FILE}.1"))), 1000);
    assert_eq!(
        fs::read_to_string(root.path().join(format!("{FILE}.2"))).unwrap(),
        "older\n"
    );
    let fresh = fs::read_to_string(&live).unwrap();
    assert_eq!(fresh.lines().count(), 1);
    assert!(fresh.ends_with(&format!(" start {}\n", "z".repeat(64))));
}

#[test]
fn interior_cr_and_lf_are_written_escaped_on_one_line() {
    let root = tempfile::tempdir().unwrap();
    let log = RotatingLog::open(config(root.path(), MAX_BYTES), redactor());
    log.record_at(at(0, 0), "multi\nline", "one\r\ntwo\nthree\r");
    log.record_json(&JsonRecord::new().text("detail", "a\r\nb"));

    let contents = fs::read_to_string(root.path().join(FILE)).unwrap();
    assert_eq!(
        contents,
        "1970-01-01T00:00:00.000Z multi\\nline one\\r\\ntwo\\nthree\\r\n{\"detail\":\"a\\r\\nb\"}\n"
    );
}

#[test]
fn an_oversized_line_is_dropped_and_counted() {
    let root = tempfile::tempdir().unwrap();
    let log = RotatingLog::open(config(root.path(), 64), redactor());
    log.record("event", &"x".repeat(200));
    assert!(!root.path().join(FILE).exists());

    log.record("event", "fits");
    assert_eq!(
        log.stats(),
        LogStats {
            written: 1,
            oversized: 1,
            failed: 0
        }
    );
    assert_eq!(
        fs::read_to_string(root.path().join(FILE))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn field_bounds_hold_before_the_line_is_built() {
    let root = tempfile::tempdir().unwrap();
    let mut config = config(root.path(), MAX_BYTES);
    config.field_bytes = 8;
    let log = RotatingLog::open(config, redactor());
    log.record_at(at(0, 0), "event-name-long", "detail-that-is-long");
    assert_eq!(
        fs::read_to_string(root.path().join(FILE)).unwrap(),
        "1970-01-01T00:00:00.000Z event-na detail-t\n"
    );
}

#[test]
fn a_failed_directory_creation_is_dropped_and_counted() {
    let root = tempfile::tempdir().unwrap();
    let blocker = root.path().join("blocker");
    fs::write(&blocker, b"not a directory").unwrap();
    let log = RotatingLog::open(config(&blocker.join("logs"), MAX_BYTES), redactor());
    log.record("event", "detail");
    log.record_json(&JsonRecord::new().text("event", "failed"));
    assert_eq!(log.stats().failed, 2);
    assert_eq!(fs::read(&blocker).unwrap(), b"not a directory");

    let empty = RotatingLog::open(config(Path::new(""), MAX_BYTES), redactor());
    empty.record("event", "detail");
    assert_eq!(empty.stats().failed, 1);
}

#[test]
fn a_failed_write_is_dropped_and_counted() {
    let root = tempfile::tempdir().unwrap();
    let live = root.path().join(FILE);
    fs::write(&live, b"kept\n").unwrap();
    let mut permissions = fs::metadata(&live).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&live, permissions).unwrap();

    let log = RotatingLog::open(config(root.path(), MAX_BYTES), redactor());
    log.record("event", "detail");
    log.record("event", "detail");
    assert_eq!(
        log.stats(),
        LogStats {
            written: 0,
            oversized: 0,
            failed: 2
        }
    );
    assert_eq!(fs::read(&live).unwrap(), b"kept\n");
}

#[test]
fn a_rotation_that_cannot_clear_the_live_file_drops_the_line() {
    let root = tempfile::tempdir().unwrap();
    let max_bytes = 256;
    // The live name is a directory that is not empty: no rotation can move
    // a file out of the way, so the line must not break the bound.
    let live = root.path().join(FILE);
    fs::create_dir(&live).unwrap();
    fs::write(live.join("inside"), b"x").unwrap();
    let mut config = config(root.path(), max_bytes);
    config.retained = 0;
    let log = RotatingLog::open(config, redactor());
    log.record("event", &"x".repeat(200));
    assert_eq!(log.stats().written, 0);
    assert_eq!(log.stats().failed, 1);
}

// --- tails and fields ------------------------------------------------------

#[test]
fn the_tail_keeps_200_lines_and_64_kib_with_the_omitted_marker() {
    let output = format!("{}last line", "old output\n".repeat(250));
    let tail = bounded_tail(&redactor(), &output, TAIL);
    assert_eq!(tail.lines().count(), 200);
    assert!(tail.ends_with("last line"));
    assert!(!tail.starts_with(OMITTED_MARKER));

    let wide = format!("{}\nlast line", "界".repeat(30_000));
    let tail = bounded_tail(&redactor(), &wide, TAIL);
    assert!(tail.len() <= TAIL.bytes);
    assert!(tail.len() > TAIL.bytes - 4);
    assert!(tail.starts_with(OMITTED_MARKER));
    assert!(tail.ends_with("\nlast line"));

    let short = bounded_tail(&redactor(), "a\r\nb\nc", TAIL);
    assert_eq!(short, "a\nb\nc");
}

#[test]
fn a_tail_cut_inside_a_private_key_block_leaves_no_key_line() {
    // Cut first, the last five lines would start inside the block and keep
    // four body lines. Redact first, the block is one marker.
    let output = format!("{}{}\ndone", "old output\n".repeat(10), pem_block());
    let bounds = TailBounds {
        lines: 5,
        bytes: 64 * 1024,
    };
    let tail = bounded_tail(&redactor(), &output, bounds);
    assert_eq!(tail, "old output\nold output\nold output\n[redacted]\ndone");

    // The byte cut too: a budget that would start mid-block.
    let bounds = TailBounds {
        lines: 200,
        bytes: OMITTED_MARKER.len() + 40,
    };
    let tail = bounded_tail(&redactor(), &output, bounds);
    for body in PEM_BODY {
        for piece in body.as_bytes().chunks(8) {
            let piece = std::str::from_utf8(piece).unwrap();
            assert!(!tail.contains(piece), "{tail}");
        }
    }
    assert!(tail.starts_with(OMITTED_MARKER));
    assert!(tail.ends_with("[redacted]\ndone"));
}

#[test]
fn cuts_next_to_multi_byte_utf8_land_on_character_boundaries() {
    // "é" is two bytes; a five-byte cut would split the third one.
    assert_eq!(bound_field(&redactor(), "ééé", 5), "éé");
    assert_eq!(bound_field(&redactor(), "ééé", 6), "ééé");
    assert_eq!(bound_field(&redactor(), "界", 2), "");

    // A tail cut moves forward past a split character.
    for extra in 0..4 {
        let bounds = TailBounds {
            lines: 10,
            bytes: OMITTED_MARKER.len() + 7 + extra,
        };
        let tail = bounded_tail(&redactor(), "界界界界界", bounds);
        assert!(tail.len() <= bounds.bytes);
        assert!(tail.starts_with(OMITTED_MARKER));
        let kept = &tail[OMITTED_MARKER.len()..];
        assert!(kept.chars().all(|character| character == '界'));
    }

    let tiny = TailBounds {
        lines: 10,
        bytes: 3,
    };
    assert_eq!(bounded_tail(&redactor(), "longer than three", tiny), "");
}

#[test]
fn fields_are_redacted_before_they_are_cut() {
    // Cut first, eight bytes would keep `ghp_abcd`, part of the token.
    let field = bound_field(&redactor(), "ghp_abcdefgh tail", 8);
    assert_eq!(field, "[redacte");
    assert!(!field.contains("ghp_"));
}
