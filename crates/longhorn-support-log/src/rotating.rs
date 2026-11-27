//! Size-bounded rotating line log on disk.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use longhorn_redact::Redactor;

use crate::text::{bound_field, push_json_string, single_line};
use crate::time::utc_timestamp;

/// Where a [`RotatingLog`] writes and how far it may grow. Every value is
/// the caller's; none has a default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RotatingLogConfig {
    /// The directory that holds the live file and its rotations. Created on
    /// the first write. An empty path never writes.
    pub directory: PathBuf,
    /// The live file's name. Rotations are `<file_name>.1` (newest) through
    /// `<file_name>.<retained>`.
    pub file_name: String,
    /// The most bytes any one file may hold.
    pub max_bytes: u64,
    /// Rotated files kept beside the live one.
    pub retained: u32,
    /// The most bytes each redacted text field keeps.
    pub field_bytes: usize,
}

/// What a [`RotatingLog`] did with its lines since it was opened. Each line
/// is counted once, by the first step that decides it, in this order: a
/// repair latch (`failed`), then `max_bytes` (`oversized`), then an empty
/// directory or an I/O failure (`failed`), otherwise a whole write
/// (`written`). A JSON record dropped for colliding keys is counted in
/// [`RotatingLog::rejected_records`] before any of these steps, and is not
/// counted here. A degraded rotation is counted in
/// [`RotatingLog::degraded_rotations`], not here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogStats {
    /// Lines written whole. A line written after a degraded rotation counts
    /// here like any other; this count does not show retention loss.
    pub written: u64,
    /// Lines larger than `max_bytes`, dropped unwritten. A latched line is
    /// counted in `failed` instead.
    pub oversized: u64,
    /// Lines dropped by the repair latch, an empty directory, a directory
    /// creation, open or write failure, or because the live file was still
    /// over the bound after rotation. A degraded rotation alone drops no
    /// line.
    pub failed: u64,
    /// Whether a failed write also failed to truncate back to the
    /// pre-append length, leaving a partial line the log cannot repair.
    /// Latched: once set it stays set for the life of the log.
    pub repair_required: bool,
}

/// One JSON Lines record. The caller names the fields; the log redacts and
/// bounds each text value before it serializes the line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JsonRecord {
    fields: Vec<(String, JsonValue)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum JsonValue {
    Text(String),
    Integer(i64),
}

impl JsonRecord {
    /// An empty record.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a text field.
    #[must_use]
    pub fn text(mut self, key: &str, value: &str) -> Self {
        self.fields
            .push((key.to_owned(), JsonValue::Text(value.to_owned())));
        self
    }

    /// Appends a text field when `value` is present.
    #[must_use]
    pub fn optional_text(self, key: &str, value: Option<&str>) -> Self {
        match value {
            Some(value) => self.text(key, value),
            None => self,
        }
    }

    /// Appends an integer field.
    #[must_use]
    pub fn integer(mut self, key: &str, value: i64) -> Self {
        self.fields
            .push((key.to_owned(), JsonValue::Integer(value)));
        self
    }

    /// Appends an integer field when `value` is present.
    #[must_use]
    pub fn optional_integer(self, key: &str, value: Option<i64>) -> Self {
        match value {
            Some(value) => self.integer(key, value),
            None => self,
        }
    }
}

/// A size-bounded rotating line log. Share one instance per file, for
/// example behind an `Arc`; writes through it are serialized.
#[derive(Debug)]
pub struct RotatingLog {
    sink: Option<Sink>,
}

#[derive(Debug)]
struct Sink {
    redactor: Redactor,
    config: RotatingLogConfig,
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    file: Option<File>,
    stats: LogStats,
    degraded_rotations: u64,
    rejected_records: u64,
}

impl RotatingLog {
    /// A log under `config`. Nothing touches the disk until the first write.
    #[must_use]
    pub fn open(config: RotatingLogConfig, redactor: Redactor) -> Self {
        Self {
            sink: Some(Sink {
                redactor,
                config,
                state: Mutex::new(State::default()),
            }),
        }
    }

    /// A log that writes nowhere and never opens a file.
    #[must_use]
    pub fn discarding() -> Self {
        Self { sink: None }
    }

    /// The directory this log writes to; `None` for a discarding log.
    #[must_use]
    pub fn directory(&self) -> Option<&Path> {
        self.sink
            .as_ref()
            .map(|sink| sink.config.directory.as_path())
    }

    /// Line outcomes since the log was opened, in [`LogStats`]. Excludes
    /// rejected JSON records ([`RotatingLog::rejected_records`]) and degraded
    /// rotations ([`RotatingLog::degraded_rotations`]). A discarding log
    /// counts nothing.
    #[must_use]
    pub fn stats(&self) -> LogStats {
        self.sink.as_ref().map_or_else(LogStats::default, |sink| {
            sink.state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .stats
        })
    }

    /// The number of rotations since this instance opened in which at least
    /// one step failed for a reason other than a missing path, each such
    /// rotation counted once. Such a rotation may have overwritten or lost a
    /// retained generation. Zero means no counted rotation-step failure since
    /// this instance opened. Normal eviction of the oldest generation, a file
    /// removed outside the log, and any other instance are not counted. The
    /// line that triggered a degraded rotation still counts in
    /// [`LogStats::written`] if it lands. A discarding log counts nothing.
    #[must_use]
    pub fn degraded_rotations(&self) -> u64 {
        self.sink.as_ref().map_or(0, |sink| {
            sink.state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .degraded_rotations
        })
    }

    /// JSON records dropped whole because two keys were equal after redaction
    /// and bounding, so a reader could not tell them apart. This check runs
    /// before the size and state checks, and these records are not part of
    /// [`RotatingLog::stats`]. A discarding log counts nothing.
    #[must_use]
    pub fn rejected_records(&self) -> u64 {
        self.sink.as_ref().map_or(0, |sink| {
            sink.state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .rejected_records
        })
    }

    /// Writes `<timestamp> <event> <detail>` now.
    pub fn record(&self, event: &str, detail: &str) {
        self.record_at(SystemTime::now(), event, detail);
    }

    /// Writes `<timestamp> <event> <detail>` stamped `at`. `event` and
    /// `detail` are each redacted, bounded, then written on one line.
    pub fn record_at(&self, at: SystemTime, event: &str, detail: &str) {
        let Some(sink) = &self.sink else {
            return;
        };
        let line = format!(
            "{} {} {}\n",
            utc_timestamp(at),
            single_line(&sink.field(event)),
            single_line(&sink.field(detail)),
        );
        sink.write(&line);
    }

    /// Writes `record` as one JSON object line. Keys and text values are
    /// each redacted, then bounded, then escaped. A record whose keys are
    /// equal after that transform is dropped whole and counted in
    /// [`RotatingLog::rejected_records`].
    pub fn record_json(&self, record: &JsonRecord) {
        let Some(sink) = &self.sink else {
            return;
        };
        match sink.json_line(record) {
            Some(line) => sink.write(&line),
            None => sink.reject(),
        }
    }
}

impl Sink {
    fn field(&self, text: &str) -> String {
        bound_field(&self.redactor, text, self.config.field_bytes)
    }

    /// Serializes `record` as one JSON line, or `None` when two keys are
    /// equal after redaction and bounding. Keys are checked after their
    /// transform, so identical keys, keys redaction makes equal and keys a
    /// bound cuts to the same head are all caught.
    fn json_line(&self, record: &JsonRecord) -> Option<String> {
        let mut seen = HashSet::new();
        let mut fields = Vec::with_capacity(record.fields.len());
        for (key, value) in &record.fields {
            let key = self.field(key);
            if !seen.insert(key.clone()) {
                return None;
            }
            fields.push((key, value));
        }
        let mut line = String::from("{");
        for (index, (key, value)) in fields.into_iter().enumerate() {
            if index > 0 {
                line.push(',');
            }
            push_json_string(&mut line, &key);
            line.push(':');
            match value {
                JsonValue::Text(text) => push_json_string(&mut line, &self.field(text)),
                JsonValue::Integer(number) => line.push_str(&number.to_string()),
            }
        }
        line.push_str("}\n");
        Some(line)
    }

    /// Counts a JSON record dropped for colliding keys. Nothing reaches disk.
    fn reject(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.rejected_records += 1;
    }

    /// Appends one whole line, rotating first when it would cross the bound.
    /// Never panics and never returns a failure; drops are counted. While
    /// a repair is required the log writes nothing: the live file may hold
    /// a partial line, so every line is dropped until the operator repairs
    /// the file and restarts with a fresh log.
    fn write(&self, line: &str) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.stats.repair_required {
            state.stats.failed += 1;
            return;
        }
        let incoming = u64::try_from(line.len()).unwrap_or(u64::MAX);
        if incoming > self.config.max_bytes {
            state.stats.oversized += 1;
            return;
        }
        if self.config.directory.as_os_str().is_empty()
            || !is_safe_file_name(&self.config.file_name)
        {
            state.stats.failed += 1;
            return;
        }
        let live = self.config.directory.join(&self.config.file_name);
        if self.over_bound(&live, incoming, &mut state) {
            // Drop the handle before the rename so the next write opens the
            // fresh file rather than appending to the rotated one.
            state.file = None;
            if !self.rotate() {
                state.degraded_rotations += 1;
            }
            if self.over_bound(&live, incoming, &mut state) {
                state.stats.failed += 1;
                return;
            }
        }
        if state.file.is_none() {
            state.file = fs::create_dir_all(&self.config.directory)
                .and_then(|()| OpenOptions::new().create(true).append(true).open(&live))
                .ok();
        }
        let Some(file) = state.file.as_mut() else {
            state.stats.failed += 1;
            return;
        };
        let outcome = append_whole_line(file, line.as_bytes());
        apply_append_outcome(&mut state, outcome);
    }

    /// Whether `incoming` bytes would push a non-empty live file past the
    /// bound. A missing live file also drops a handle that still points at
    /// a removed one.
    fn over_bound(&self, live: &Path, incoming: u64, state: &mut State) -> bool {
        match fs::metadata(live) {
            Ok(metadata) => {
                metadata.len() > 0
                    && metadata.len().saturating_add(incoming) > self.config.max_bytes
            }
            Err(_) => {
                state.file = None;
                false
            }
        }
    }

    /// Removes the oldest generation, shifts the rest up one, then moves the
    /// live file to `.1`. With nothing retained, removes the live file. Every
    /// step runs even after an earlier one fails, because the later steps
    /// still move what they can. Returns `false` when a step failed for a
    /// reason other than a missing path. A generation that could not shift
    /// is then overwritten by the one above it, so the caller records the
    /// degradation.
    fn rotate(&self) -> bool {
        let directory = &self.config.directory;
        let name = &self.config.file_name;
        let live = directory.join(name);
        let retained = self.config.retained;
        if retained == 0 {
            return settled(fs::remove_file(live));
        }
        let rotated = |generation: u32| directory.join(format!("{name}.{generation}"));
        let mut clean = settled(fs::remove_file(rotated(retained)));
        for generation in (1..retained).rev() {
            clean &= settled(fs::rename(rotated(generation), rotated(generation + 1)));
        }
        // Computed before the `&&`, so the live file moves even after a failed shift.
        let moved = settled(fs::rename(live, rotated(1)));
        clean && moved
    }
}

/// Whether `file_name` is a single normal path component within `directory`.
fn is_safe_file_name(file_name: &str) -> bool {
    if file_name.is_empty() || matches!(file_name, "." | "..") {
        return false;
    }
    if file_name.contains(['/', '\\', '\0']) {
        return false;
    }
    let mut components = Path::new(file_name).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

/// Whether a rotation step finished. A missing path is not a failure: a log
/// with fewer rotations than `retained` has no older generation to move.
fn settled(result: io::Result<()>) -> bool {
    match result {
        Ok(()) => true,
        Err(error) => error.kind() == io::ErrorKind::NotFound,
    }
}

/// One whole-line append attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppendOutcome {
    /// The line reached the stream whole.
    Written,
    /// The write failed and the stream is back at its pre-append length,
    /// so the next line cannot join onto a partial one.
    Dropped,
    /// The write failed and the stream would not truncate back. It may
    /// hold a partial line; the operator repairs it.
    RepairRequired,
}

/// A stream an append can measure and truncate. `File` is the only
/// production target; tests inject a fake.
trait AppendTarget: Write {
    fn committed_len(&mut self) -> io::Result<u64>;
    fn truncate_to(&mut self, len: u64) -> io::Result<()>;
}

impl AppendTarget for File {
    fn committed_len(&mut self) -> io::Result<u64> {
        self.metadata().map(|metadata| metadata.len())
    }

    fn truncate_to(&mut self, len: u64) -> io::Result<()> {
        self.set_len(len)
    }
}

/// Records one append outcome in `state`. A failure drops the handle so
/// the next write reopens; a repair-required failure also latches
/// `repair_required`, which [`Sink::write`] honors by dropping every later
/// line until the operator repairs the file.
fn apply_append_outcome(state: &mut State, outcome: AppendOutcome) {
    match outcome {
        AppendOutcome::Written => {
            state.stats.written += 1;
        }
        AppendOutcome::Dropped => {
            state.file = None;
            state.stats.failed += 1;
        }
        AppendOutcome::RepairRequired => {
            state.file = None;
            state.stats.failed += 1;
            state.stats.repair_required = true;
        }
    }
}

/// Appends `line` whole or not at all. A writer may commit a prefix before
/// it fails (`write_all` returns the error, not the bytes), so on any
/// failure the stream is truncated back to its pre-append length. When the
/// truncation fails too the stream may hold a partial line and the caller
/// must report repair.
fn append_whole_line(target: &mut impl AppendTarget, line: &[u8]) -> AppendOutcome {
    let pre_len = target.committed_len().ok();
    if target.write_all(line).and_then(|()| target.flush()).is_ok() {
        return AppendOutcome::Written;
    }
    let repaired = match pre_len {
        Some(len) => target
            .truncate_to(len)
            .and_then(|()| target.flush())
            .is_ok(),
        None => false,
    };
    if repaired {
        AppendOutcome::Dropped
    } else {
        AppendOutcome::RepairRequired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An injected writer that commits a `fail_after`-byte prefix of each
    /// append, then fails; it optionally refuses to truncate.
    struct FailingWriter {
        committed: Vec<u8>,
        fail_after: usize,
        truncate_fails: bool,
    }

    impl FailingWriter {
        fn seeded(fail_after: usize) -> Self {
            Self {
                committed: b"good line\n".to_vec(),
                fail_after: b"good line\n".len() + fail_after,
                truncate_fails: false,
            }
        }
    }

    impl Write for FailingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let remaining = self.fail_after.saturating_sub(self.committed.len());
            if remaining == 0 {
                return Err(io::Error::other("injected write failure"));
            }
            let taken = remaining.min(buf.len());
            self.committed.extend_from_slice(&buf[..taken]);
            Ok(taken)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl AppendTarget for FailingWriter {
        fn committed_len(&mut self) -> io::Result<u64> {
            Ok(self.committed.len() as u64)
        }

        fn truncate_to(&mut self, len: u64) -> io::Result<()> {
            if self.truncate_fails {
                return Err(io::Error::other("injected truncate failure"));
            }
            self.committed.truncate(len as usize);
            Ok(())
        }
    }

    #[test]
    fn a_short_write_is_truncated_back_to_the_pre_append_length() {
        let mut writer = FailingWriter::seeded(4);
        assert_eq!(
            append_whole_line(&mut writer, b"{\"event\":\"failed\"}\n"),
            AppendOutcome::Dropped
        );
        // The four committed prefix bytes are gone: the next line cannot
        // join onto a partial record.
        assert_eq!(writer.committed, b"good line\n");
    }

    #[test]
    fn a_write_that_fails_before_any_byte_is_dropped_whole() {
        let mut writer = FailingWriter::seeded(0);
        assert_eq!(
            append_whole_line(&mut writer, b"{\"event\":\"failed\"}\n"),
            AppendOutcome::Dropped
        );
        assert_eq!(writer.committed, b"good line\n");
    }

    #[test]
    fn a_failed_truncate_reports_repair_required() {
        let mut writer = FailingWriter::seeded(4);
        writer.truncate_fails = true;
        assert_eq!(
            append_whole_line(&mut writer, b"{\"event\":\"failed\"}\n"),
            AppendOutcome::RepairRequired
        );
        // The partial prefix is still there for the operator to repair.
        assert_eq!(writer.committed, b"good line\n{\"ev");
    }

    #[test]
    fn a_whole_write_is_kept() {
        let mut writer = FailingWriter::seeded(usize::MAX / 2);
        assert_eq!(
            append_whole_line(&mut writer, b"{\"event\":\"ok\"}\n"),
            AppendOutcome::Written
        );
        assert_eq!(writer.committed, b"good line\n{\"event\":\"ok\"}\n");
    }

    #[test]
    fn outcomes_count_and_latch_repair() {
        let mut state = State::default();
        apply_append_outcome(&mut state, AppendOutcome::Written);
        apply_append_outcome(&mut state, AppendOutcome::Dropped);
        assert_eq!(state.stats.written, 1);
        assert_eq!(state.stats.failed, 1);
        assert!(!state.stats.repair_required);

        apply_append_outcome(&mut state, AppendOutcome::RepairRequired);
        assert_eq!(state.stats.failed, 2);
        assert!(state.stats.repair_required);
    }

    #[test]
    fn a_latched_repair_drops_later_lines_without_touching_the_disk() {
        use longhorn_redact::{RedactionRules, Redactor};

        let root = tempfile::tempdir().unwrap();
        let sink = Sink {
            redactor: Redactor::new(Vec::new(), RedactionRules::common()),
            config: RotatingLogConfig {
                directory: root.path().to_path_buf(),
                file_name: "support.log".to_owned(),
                max_bytes: 1024,
                retained: 2,
                field_bytes: 1024,
            },
            state: Mutex::new(State {
                stats: LogStats {
                    repair_required: true,
                    ..LogStats::default()
                },
                ..State::default()
            }),
        };
        sink.write("{\"event\":\"after\"}\n");
        let state = sink.state.lock().unwrap();
        assert_eq!(state.stats.written, 0);
        assert_eq!(state.stats.failed, 1);
        assert!(state.stats.repair_required);
        // No partial record is joined: nothing was opened or created.
        assert!(!root.path().join("support.log").exists());
    }

    /// A log whose sink is built directly, so a test can latch it or point it
    /// at an empty directory without a failing writer.
    fn sink_in(directory: &Path, max_bytes: u64, latched: bool) -> RotatingLog {
        use longhorn_redact::{RedactionRules, Redactor};

        RotatingLog {
            sink: Some(Sink {
                redactor: Redactor::new(Vec::new(), RedactionRules::common()),
                config: RotatingLogConfig {
                    directory: directory.to_path_buf(),
                    file_name: "support.log".to_owned(),
                    max_bytes,
                    retained: 2,
                    field_bytes: 1024,
                },
                state: Mutex::new(State {
                    stats: LogStats {
                        repair_required: latched,
                        ..LogStats::default()
                    },
                    ..State::default()
                }),
            }),
        }
    }

    #[test]
    fn a_latched_oversized_line_counts_as_failed_not_oversized() {
        let root = tempfile::tempdir().unwrap();
        let log = sink_in(root.path(), 64, true);
        log.record_at(SystemTime::UNIX_EPOCH, "event", &"x".repeat(200));
        let stats = log.stats();
        assert_eq!((stats.failed, stats.oversized), (1, 0));
        assert!(!root.path().join("support.log").exists());
    }

    #[test]
    fn an_oversized_line_counts_as_oversized_before_the_empty_directory() {
        let log = sink_in(Path::new(""), 64, false);
        log.record_at(SystemTime::UNIX_EPOCH, "event", &"x".repeat(200));
        let stats = log.stats();
        assert_eq!((stats.oversized, stats.failed), (1, 0));
    }

    #[test]
    fn a_colliding_record_is_rejected_before_the_latch_and_the_size_bound() {
        let root = tempfile::tempdir().unwrap();
        let log = sink_in(root.path(), 64, true);
        log.record_json(
            &JsonRecord::new()
                .text("event", "a")
                .text("event", &"x".repeat(200)),
        );
        assert_eq!(log.rejected_records(), 1);
        let stats = log.stats();
        assert_eq!((stats.failed, stats.oversized, stats.written), (0, 0, 0));
        assert!(!root.path().join("support.log").exists());
    }
}
