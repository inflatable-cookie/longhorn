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
        let envelope = json!({
            "sequence": self.sequence.fetch_add(1, Ordering::Relaxed) + 1,
            "unix_millis": unix_millis(),
            "event": event,
            "detail": detail,
        });
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| "evidence lock is poisoned".to_string())?;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CheckStatus {
    Pass,
    Unmet,
    Fail,
}

#[derive(Debug, Serialize)]
pub(crate) struct Check {
    id: &'static str,
    status: CheckStatus,
    detail: Value,
}

impl Check {
    pub(crate) fn new(id: &'static str, status: CheckStatus, detail: Value) -> Self {
        Self { id, status, detail }
    }

    const fn status(&self) -> CheckStatus {
        self.status
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ProofReport {
    schema: &'static str,
    outcome: &'static str,
    platform: &'static str,
    architecture: &'static str,
    tauri_version: &'static str,
    evidence_root: PathBuf,
    checks: Vec<Check>,
}

impl ProofReport {
    pub(crate) fn completed(evidence_root: PathBuf, checks: Vec<Check>) -> Self {
        let outcome = if checks
            .iter()
            .any(|check| check.status() == CheckStatus::Fail)
        {
            "failed"
        } else if checks
            .iter()
            .any(|check| check.status() == CheckStatus::Unmet)
        {
            "pass_with_unmet_environment_claims"
        } else {
            "pass"
        };
        Self {
            schema: "longhorn.native-content.backing-surface-proof.v1",
            outcome,
            platform: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            tauri_version: "2.10.3",
            evidence_root,
            checks,
        }
    }

    pub(crate) fn failed(&self) -> bool {
        self.outcome == "failed"
    }
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
