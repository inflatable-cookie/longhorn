//! Contract 025 (`longhorn-support-log`): a ring log and a rotating log, bound
//! to a seeded redactor, with the redact-then-bound order and the rotation
//! bound asserted.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use longhorn_redact::{RedactionRules, Redactor};
use longhorn_support_log::{
    JsonRecord, Level, OMITTED_MARKER, RingLog, RotatingLog, RotatingLogConfig, TailBounds,
    bounded_tail, utc_timestamp,
};
use tempfile::TempDir;

/// A seeded secret. It stands for whatever a product decides is sensitive.
const SEED: &str = "seeded-product-credential";

/// A PEM private-key block. The redactor turns the whole block into one marker
/// before any bound cuts, so no cut can start inside it.
const KEY_BLOCK: &str =
    "-----BEGIN RSA PRIVATE KEY-----\nprivate-body-line\n-----END RSA PRIVATE KEY-----";

fn redactor() -> Redactor {
    Redactor::new([SEED.to_owned()], RedactionRules::common())
}

/// 2023-11-14T22:13:20Z. Record offsets are measured from it so the rendered
/// lines read like a real log.
const BASE_MILLISECONDS: u64 = 1_700_000_000_000;

fn at(offset_milliseconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(BASE_MILLISECONDS + offset_milliseconds)
}

pub(crate) fn run() -> String {
    // RFC 3339, UTC, exactly three fraction digits, truncated toward the past.
    assert_eq!(
        utc_timestamp(at(123)),
        "2023-11-14T22:13:20.123Z",
        "utc_timestamp format"
    );

    // A ring log keeps its capacity and drops the oldest record first.
    let ring = RingLog::new(3, 200, redactor());
    ring.record_at(at(1_000), Level::Info, "install", "step one");
    ring.record_at(
        at(2_000),
        Level::Warning,
        "install",
        "token=seeded-product-credential rejected",
    );
    ring.record_at(
        at(3_000),
        Level::Error,
        "clone",
        &format!("remote said: {KEY_BLOCK}"),
    );
    ring.record_at(at(4_000), Level::Info, "install", "step two");
    assert_eq!(ring.len(), 3, "capacity holds three records");
    let kept = ring.tail(3);
    assert_eq!(kept.len(), 3, "tail returns what is held");
    assert_eq!(kept[0].at(), at(2_000), "the oldest record was evicted");
    assert_eq!(kept[0].message(), "token=[redacted] rejected");
    // A PEM block's span starts at the beginning of the line that holds its
    // BEGIN marker, so the text before it on that line goes with it.
    assert_eq!(kept[1].message(), "[redacted]");
    assert_eq!(kept[2].message(), "step two");
    assert_eq!(
        kept[1].line(),
        "2023-11-14T22:13:23.000Z [error] clone: [redacted]",
        "ring line shape"
    );
    assert_eq!(ring.tail(1)[0].level(), Level::Info, "tail keeps order");

    // Redact, then bound: a field bound may split a marker, and never exposes
    // part of the key whose block the redactor replaced.
    let cut = RingLog::new(1, 12, redactor());
    cut.record_at(
        at(5_000),
        Level::Error,
        "install",
        &format!("start\n{KEY_BLOCK}"),
    );
    let record = &cut.tail(1)[0];
    assert_eq!(
        record.message(),
        "start\n[redac",
        "the field bound cuts after redaction"
    );
    assert!(!record.message().contains("private-body-line"));

    // `bounded_tail` redacts first, takes the last lines, then the newest
    // bytes behind the omitted marker.
    let text = format!("first line\nsecond line with {SEED}\n{KEY_BLOCK}\nlast line");
    let tail = bounded_tail(
        &redactor(),
        &text,
        TailBounds {
            lines: 3,
            bytes: 400,
        },
    );
    assert_eq!(
        tail, "second line with [redacted]\n[redacted]\nlast line",
        "tail lines after redaction"
    );
    let omitted = bounded_tail(
        &redactor(),
        &text,
        TailBounds {
            lines: 10,
            bytes: 40,
        },
    );
    assert!(omitted.len() <= 40, "the byte bound holds");
    assert!(omitted.starts_with(OMITTED_MARKER), "the cut is marked");
    assert!(
        !omitted.contains("private-body-line"),
        "no key body survives"
    );

    // A rotating log writes whole lines, never over its bound, and rotates
    // the live file into retained generations.
    let root: TempDir = tempfile::tempdir().expect("a rotating-log directory");
    let directory = root.path().join("logs");
    let log = RotatingLog::open(
        RotatingLogConfig {
            directory: directory.clone(),
            file_name: "support.log".to_owned(),
            max_bytes: 200,
            retained: 2,
            field_bytes: 400,
        },
        redactor(),
    );
    assert_eq!(log.directory(), Some(directory.as_path()));
    for index in 0..12 {
        log.record_at(
            at(1_000 + index),
            "install",
            &format!("line {index} token={SEED}"),
        );
    }
    assert_eq!(log.stats().written, 12, "every line fit the rotation bound");
    assert_eq!(log.stats().oversized, 0);
    assert_eq!(log.stats().failed, 0);

    let mut names = std::fs::read_dir(&directory)
        .expect("the log directory exists")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        vec![
            "support.log".to_owned(),
            "support.log.1".to_owned(),
            "support.log.2".to_owned()
        ],
        "two retained generations and the live file"
    );
    for name in &names {
        let bytes = std::fs::metadata(directory.join(name))
            .expect("a log file")
            .len();
        assert!(bytes <= 200, "{name} is over its bound: {bytes} bytes");
    }
    let live = std::fs::read_to_string(directory.join("support.log")).expect("the live file");
    assert_eq!(live.lines().count(), 3, "the live file holds whole lines");
    for line in live.lines() {
        assert!(
            line.ends_with("token=[redacted]"),
            "line is redacted: {line}"
        );
    }
    assert!(!live.contains(SEED), "the live file holds no seed");
    assert!(!live.contains("token=seeded"), "no seed prefix survives");
    for index in [9, 10, 11] {
        assert!(
            live.contains(&format!("line {index} ")),
            "the live file holds the newest lines"
        );
    }
    assert!(
        !live.contains("line 8 "),
        "the live file starts after line 8"
    );
    let oldest = std::fs::read_to_string(directory.join("support.log.2")).expect("generation 2");
    assert!(
        oldest.contains("line 4 "),
        "generation 2 holds the older lines"
    );
    assert!(
        !oldest.contains("line 7 "),
        "a generation does not leak forward"
    );

    // A line over the file bound is dropped whole and counted.
    let live_size = std::fs::metadata(directory.join("support.log"))
        .expect("the live file")
        .len();
    log.record("install", &"x".repeat(300));
    assert_eq!(log.stats().oversized, 1, "an oversized line is counted");
    assert_eq!(log.stats().written, 12, "an oversized line is not written");
    assert_eq!(
        std::fs::metadata(directory.join("support.log"))
            .expect("the live file")
            .len(),
        live_size,
        "the live file did not grow"
    );

    // A JSON record is redacted per value, then serialized as one line.
    let json_root = tempfile::tempdir().expect("a JSON log directory");
    let json_log = RotatingLog::open(
        RotatingLogConfig {
            directory: json_root.path().join("logs"),
            file_name: "attempts.jsonl".to_owned(),
            max_bytes: 4_096,
            retained: 1,
            field_bytes: 400,
        },
        redactor(),
    );
    json_log.record_json(
        &JsonRecord::new()
            .text("event", "install")
            .text("detail", &format!("token={SEED}"))
            .integer("attempt", 2)
            .optional_text("scope", None)
            .optional_integer("exit", Some(7)),
    );
    let line = std::fs::read_to_string(json_root.path().join("logs/attempts.jsonl"))
        .expect("the JSON line");
    assert_eq!(
        line, "{\"event\":\"install\",\"detail\":\"token=[redacted]\",\"attempt\":2,\"exit\":7}\n",
        "JSON line"
    );
    assert_eq!(json_log.stats().written, 1, "one JSON line was written");

    format!(
        "ring capacity, eviction and line shape; redact-then-bound with a key block cut; tail bound with {OMITTED_MARKER:?}; rotation at 200 bytes keeping 2 generations; oversized drop; JSON line"
    )
}
