//! Fixed-capacity in-memory ring log.

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use longhorn_redact::Redactor;

use crate::text::{bound_field, single_line};
use crate::time::utc_timestamp;

/// A ring record's severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// A failure.
    Error,
    /// A degraded but continuing state.
    Warning,
    /// Context.
    Info,
}

impl Level {
    /// The lower-case label a line carries.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// One redacted, bounded ring record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RingRecord {
    at: SystemTime,
    level: Level,
    scope: String,
    message: String,
}

impl RingRecord {
    /// When the record was made.
    #[must_use]
    pub fn at(&self) -> SystemTime {
        self.at
    }

    /// The record's severity.
    #[must_use]
    pub fn level(&self) -> Level {
        self.level
    }

    /// The redacted, bounded scope.
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// The redacted, bounded message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// One line: `2026-10-01T12:00:00.000Z [error] github: detail`, with
    /// interior CR and LF written as `\r` and `\n`.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "{} [{}] {}: {}",
            utc_timestamp(self.at),
            self.level.label(),
            single_line(&self.scope),
            single_line(&self.message),
        )
    }
}

/// A fixed-capacity, oldest-first in-memory log. Once full, the oldest
/// record leaves as the newest arrives.
#[derive(Debug)]
pub struct RingLog {
    redactor: Redactor,
    capacity: usize,
    field_bytes: usize,
    records: Mutex<VecDeque<RingRecord>>,
}

impl RingLog {
    /// A log that keeps `capacity` records. Each scope and message is
    /// redacted, then bounded to `field_bytes`.
    #[must_use]
    pub fn new(capacity: usize, field_bytes: usize, redactor: Redactor) -> Self {
        Self {
            redactor,
            capacity,
            field_bytes,
            records: Mutex::new(VecDeque::with_capacity(capacity)),
        }
    }

    /// Records now.
    pub fn record(&self, level: Level, scope: &str, message: &str) {
        self.record_at(SystemTime::now(), level, scope, message);
    }

    /// Records at `at`.
    pub fn record_at(&self, at: SystemTime, level: Level, scope: &str, message: &str) {
        if self.capacity == 0 {
            return;
        }
        let record = RingRecord {
            at,
            level,
            scope: bound_field(&self.redactor, scope, self.field_bytes),
            message: bound_field(&self.redactor, message, self.field_bytes),
        };
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        while records.len() >= self.capacity {
            records.pop_front();
        }
        records.push_back(record);
    }

    /// The newest `limit` records, oldest first.
    #[must_use]
    pub fn tail(&self, limit: usize) -> Vec<RingRecord> {
        let records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        let start = records.len().saturating_sub(limit);
        records.iter().skip(start).cloned().collect()
    }

    /// How many records the log holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// Whether the log holds no record.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
