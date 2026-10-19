//! Size-bounded rotating line log on disk.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
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

/// What a [`RotatingLog`] did with its lines since it was opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogStats {
    /// Lines written whole.
    pub written: u64,
    /// Lines larger than `max_bytes`, dropped unwritten.
    pub oversized: u64,
    /// Lines dropped because a directory creation, open, rotation or write
    /// failed.
    pub failed: u64,
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

    /// Counts since the log was opened. A discarding log counts nothing.
    #[must_use]
    pub fn stats(&self) -> LogStats {
        self.sink.as_ref().map_or_else(LogStats::default, |sink| {
            sink.state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .stats
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
    /// each redacted, then bounded, then escaped.
    pub fn record_json(&self, record: &JsonRecord) {
        let Some(sink) = &self.sink else {
            return;
        };
        let mut line = String::from("{");
        for (index, (key, value)) in record.fields.iter().enumerate() {
            if index > 0 {
                line.push(',');
            }
            push_json_string(&mut line, &sink.field(key));
            line.push(':');
            match value {
                JsonValue::Text(text) => push_json_string(&mut line, &sink.field(text)),
                JsonValue::Integer(number) => line.push_str(&number.to_string()),
            }
        }
        line.push_str("}\n");
        sink.write(&line);
    }
}

impl Sink {
    fn field(&self, text: &str) -> String {
        bound_field(&self.redactor, text, self.config.field_bytes)
    }

    /// Appends one whole line, rotating first when it would cross the bound.
    /// Never panics and never returns a failure; drops are counted.
    fn write(&self, line: &str) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let incoming = u64::try_from(line.len()).unwrap_or(u64::MAX);
        if incoming > self.config.max_bytes {
            state.stats.oversized += 1;
            return;
        }
        if self.config.directory.as_os_str().is_empty() {
            state.stats.failed += 1;
            return;
        }
        let live = self.config.directory.join(&self.config.file_name);
        if self.over_bound(&live, incoming, &mut state) {
            // Drop the handle before the rename so the next write opens the
            // fresh file rather than appending to the rotated one.
            state.file = None;
            self.rotate();
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
        if file
            .write_all(line.as_bytes())
            .and_then(|()| file.flush())
            .is_ok()
        {
            state.stats.written += 1;
        } else {
            state.file = None;
            state.stats.failed += 1;
        }
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
    /// live file to `.1`. With nothing retained, removes the live file.
    fn rotate(&self) {
        let directory = &self.config.directory;
        let name = &self.config.file_name;
        let live = directory.join(name);
        let retained = self.config.retained;
        if retained == 0 {
            let _ = fs::remove_file(live);
            return;
        }
        let rotated = |generation: u32| directory.join(format!("{name}.{generation}"));
        let _ = fs::remove_file(rotated(retained));
        for generation in (1..retained).rev() {
            let _ = fs::rename(rotated(generation), rotated(generation + 1));
        }
        let _ = fs::rename(live, rotated(1));
    }
}
