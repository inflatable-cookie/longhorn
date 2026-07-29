//! Structured packaged-run evidence.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::{Value, json};

pub(crate) struct EvidenceLog {
    root: PathBuf,
    transcript: PathBuf,
    report: PathBuf,
    sequence: AtomicU64,
    write_lock: Mutex<()>,
}

impl EvidenceLog {
    pub(crate) fn new(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        Ok(Self {
            transcript: root.join("runtime-transcript.jsonl"),
            report: root.join("final-report.json"),
            root,
            sequence: AtomicU64::new(0),
            write_lock: Mutex::new(()),
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn report_path(&self) -> &Path {
        &self.report
    }

    pub(crate) fn record(&self, event: &str, detail: Value) -> Result<(), String> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| "evidence log lock is poisoned".to_string())?;
        let envelope = json!({
            "sequence": self.sequence.fetch_add(1, Ordering::Relaxed) + 1,
            "unix_millis": unix_millis(),
            "event": event,
            "detail": detail,
        });
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.transcript)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut file, &envelope).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())
    }

    pub(crate) fn write_report(&self, report: &impl Serialize) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
        let temporary = self.report.with_extension("json.tmp");
        fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
        fs::rename(&temporary, &self.report).map_err(|error| error.to_string())
    }
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
