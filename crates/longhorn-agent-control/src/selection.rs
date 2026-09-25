//! Per-instance pending-selection registry (contract 022).
//!
//! One registry belongs to one control-surface instance. Agent-originated
//! `open`/`save` calls begin a request here; MCP `answer_selection` /
//! `reject_selection` settle it exactly once. Resource reads return the
//! still-pending set so a late subscriber can discover a request created
//! before it joined. Expiry, cancellation, and shutdown settle the waiter
//! so the JS caller is never left awaiting indefinitely.

use std::{
    collections::HashMap,
    fmt, fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::{SelectionId, ToolError};

/// Bound an unanswered request lives before it expires with a typed error.
pub const DEFAULT_SELECTION_TTL: Duration = Duration::from_secs(60);

/// Dialog filter copied from the caller's picker options.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SelectionFilter {
    /// Filter display name.
    pub name: String,
    /// File extensions without a leading dot.
    pub extensions: Vec<String>,
}

/// Open vs save. Directory intent is [`BeginSelectionRequest::directory`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionKind {
    /// Tauri JS `open`.
    Open,
    /// Tauri JS `save`. Longhorn never writes the chosen path.
    Save,
}

/// What one picker call wants before it is assigned an id.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BeginSelectionRequest {
    /// Open or save.
    pub kind: SelectionKind,
    /// Directory intent; only meaningful for [`SelectionKind::Open`].
    #[serde(default)]
    pub directory: bool,
    /// Multiple paths; only meaningful for [`SelectionKind::Open`].
    #[serde(default)]
    pub multiple: bool,
    /// Optional extension filters. Hints for the agent; not enforced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<SelectionFilter>,
    /// Dialog title, when the caller supplied one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Default-path hint, never an answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    /// Requesting window label, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    /// Requesting webview label, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webview: Option<String>,
}

/// One still-pending request as published on the selection resource.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PendingSelection {
    /// Request id the agent answers with.
    pub id: SelectionId,
    /// Instance that owns this request (`appId:pid`).
    pub instance: String,
    /// Open or save.
    pub kind: SelectionKind,
    /// Directory intent.
    #[serde(default)]
    pub directory: bool,
    /// Multiple-path intent.
    #[serde(default)]
    pub multiple: bool,
    /// Optional extension filters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<SelectionFilter>,
    /// Dialog title, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Default-path hint, never an answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    /// Requesting window label, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    /// Requesting webview label, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webview: Option<String>,
}

/// Body of `longhorn://agent-control/selection`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SelectionResource {
    /// Still-pending requests, newest last.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<PendingSelection>,
}

/// How a pending request resolved for the JS waiter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionOutcome {
    /// Agent supplied paths. Plugin-compatible delivery is the host's job.
    Answered(Vec<String>),
    /// Explicit reject: the plugin's cancellation result (`null`).
    Rejected,
    /// TTL elapsed before settlement.
    Expired,
    /// The JS waiter was dropped, or the host cancelled.
    Cancelled,
    /// The control server is shutting down.
    Shutdown,
}

/// Failure to begin a request (before it is pending).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeginSelectionError {
    /// Operating-system CSPRNG was unavailable for the request id.
    EntropyUnavailable,
}

impl fmt::Display for BeginSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntropyUnavailable => formatter.write_str("selection id entropy unavailable"),
        }
    }
}

impl std::error::Error for BeginSelectionError {}

enum SlotState {
    Pending {
        spec: PendingSelection,
        tx: oneshot::Sender<SelectionOutcome>,
        deadline: Instant,
    },
    Settled,
    Expired,
}

struct Inner {
    instance: String,
    ttl: Duration,
    generation: AtomicU64,
    slots: Mutex<HashMap<SelectionId, SlotState>>,
}

/// Per-instance pending-selection registry.
#[derive(Clone, Debug)]
pub struct SelectionRegistry {
    inner: Arc<Inner>,
}

impl fmt::Debug for Inner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SelectionRegistryInner")
            .field("instance", &self.instance)
            .field("ttl", &self.ttl)
            .field("generation", &self.generation.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl SelectionRegistry {
    /// Registry for `instance` (`appId:pid`) with the default TTL.
    #[must_use]
    pub fn new(instance: impl Into<String>) -> Self {
        Self::with_ttl(instance, DEFAULT_SELECTION_TTL)
    }

    /// Registry with an explicit TTL (tests).
    #[must_use]
    pub fn with_ttl(instance: impl Into<String>, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Inner {
                instance: instance.into(),
                ttl,
                generation: AtomicU64::new(0),
                slots: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Identity this registry will stamp on pending records.
    #[must_use]
    pub fn instance(&self) -> &str {
        &self.inner.instance
    }

    /// Monotonic generation: bumped on begin and every settlement.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::SeqCst)
    }

    /// Still-pending requests, after sweeping expired ones.
    #[must_use]
    pub fn pending(&self) -> Vec<PendingSelection> {
        self.sweep();
        let slots = self.inner.slots.lock().expect("selection registry mutex");
        slots
            .values()
            .filter_map(|slot| match slot {
                SlotState::Pending { spec, .. } => Some(spec.clone()),
                SlotState::Settled | SlotState::Expired => None,
            })
            .collect()
    }

    /// JSON body for the selection resource.
    #[must_use]
    pub fn resource_body(&self) -> SelectionResource {
        SelectionResource {
            pending: self.pending(),
        }
    }

    /// Publish a pending request and return a waiter the JS caller awaits.
    pub fn begin(
        &self,
        request: BeginSelectionRequest,
    ) -> Result<SelectionWait, BeginSelectionError> {
        self.sweep();
        let id = generate_id()?;
        let spec = PendingSelection {
            id: id.clone(),
            instance: self.inner.instance.clone(),
            kind: request.kind,
            directory: request.directory,
            multiple: request.multiple,
            filters: request.filters,
            title: request.title,
            default_path: request.default_path,
            window: request.window,
            webview: request.webview,
        };
        let (tx, rx) = oneshot::channel();
        let deadline = Instant::now() + self.inner.ttl;
        {
            let mut slots = self.inner.slots.lock().expect("selection registry mutex");
            slots.insert(id.clone(), SlotState::Pending { spec, tx, deadline });
        }
        self.bump();
        Ok(SelectionWait {
            id: id.clone(),
            registry: self.clone(),
            timeout: Box::pin(tokio::time::sleep(self.inner.ttl)),
            rx,
            settled: false,
        })
    }

    /// Agent answer: validate cardinality and path kind, then settle once.
    pub fn answer(&self, id: &SelectionId, paths: &[String]) -> Result<(), ToolError> {
        self.sweep();
        let mut slots = self.inner.slots.lock().expect("selection registry mutex");
        match slots.get(id) {
            None => {
                return Err(ToolError::UnknownSelection { id: id.clone() });
            }
            Some(SlotState::Settled) => {
                return Err(ToolError::SettledSelection { id: id.clone() });
            }
            Some(SlotState::Expired) => {
                return Err(ToolError::ExpiredSelection { id: id.clone() });
            }
            Some(SlotState::Pending { spec, .. }) => {
                validate_answer(spec, paths)?;
            }
        }
        let Some(SlotState::Pending { tx, .. }) = slots.insert(id.clone(), SlotState::Settled)
        else {
            return Err(ToolError::UnknownSelection { id: id.clone() });
        };
        drop(slots);
        let _ = tx.send(SelectionOutcome::Answered(paths.to_vec()));
        self.bump();
        Ok(())
    }

    /// Explicit reject: the JS call resolves as plugin-compatible `null`.
    pub fn reject(&self, id: &SelectionId) -> Result<(), ToolError> {
        self.settle_terminal(id, SelectionOutcome::Rejected, SlotState::Settled)
    }

    /// Cancel a still-pending waiter (host drop, not an MCP reject).
    pub fn cancel(&self, id: &SelectionId) {
        let _ = self.settle_terminal(id, SelectionOutcome::Cancelled, SlotState::Settled);
    }

    /// Settle every pending request. Idempotent.
    pub fn shutdown(&self) {
        let mut slots = self.inner.slots.lock().expect("selection registry mutex");
        let ids: Vec<_> = slots.keys().cloned().collect();
        let mut bumped = false;
        for id in ids {
            if let Some(SlotState::Pending { tx, .. }) = slots.insert(id, SlotState::Settled) {
                let _ = tx.send(SelectionOutcome::Shutdown);
                bumped = true;
            }
        }
        drop(slots);
        if bumped {
            self.bump();
        }
    }

    fn expire(&self, id: &SelectionId) {
        let _ = self.settle_terminal(id, SelectionOutcome::Expired, SlotState::Expired);
    }

    fn settle_terminal(
        &self,
        id: &SelectionId,
        outcome: SelectionOutcome,
        tombstone: SlotState,
    ) -> Result<(), ToolError> {
        self.sweep();
        let mut slots = self.inner.slots.lock().expect("selection registry mutex");
        match slots.get(id) {
            None => return Err(ToolError::UnknownSelection { id: id.clone() }),
            Some(SlotState::Settled) => {
                return Err(ToolError::SettledSelection { id: id.clone() });
            }
            Some(SlotState::Expired) => {
                return Err(ToolError::ExpiredSelection { id: id.clone() });
            }
            Some(SlotState::Pending { .. }) => {}
        }
        let Some(SlotState::Pending { tx, .. }) = slots.insert(id.clone(), tombstone) else {
            return Err(ToolError::UnknownSelection { id: id.clone() });
        };
        drop(slots);
        let _ = tx.send(outcome);
        self.bump();
        Ok(())
    }

    fn sweep(&self) {
        let now = Instant::now();
        let mut slots = self.inner.slots.lock().expect("selection registry mutex");
        let expired: Vec<_> = slots
            .iter()
            .filter_map(|(id, slot)| match slot {
                SlotState::Pending { deadline, .. } if *deadline <= now => Some(id.clone()),
                _ => None,
            })
            .collect();
        let mut bumped = false;
        for id in expired {
            if let Some(SlotState::Pending { tx, .. }) = slots.insert(id, SlotState::Expired) {
                let _ = tx.send(SelectionOutcome::Expired);
                bumped = true;
            }
        }
        drop(slots);
        if bumped {
            self.bump();
        }
    }

    fn bump(&self) {
        self.inner.generation.fetch_add(1, Ordering::SeqCst);
    }
}

/// Waiter the JS/host caller holds until the request settles.
///
/// Dropping without awaiting cancels the request. Awaiting past the TTL
/// expires it even if the agent never answers.
pub struct SelectionWait {
    id: SelectionId,
    registry: SelectionRegistry,
    timeout: Pin<Box<tokio::time::Sleep>>,
    rx: oneshot::Receiver<SelectionOutcome>,
    settled: bool,
}

impl SelectionWait {
    /// Request id published on the selection resource.
    #[must_use]
    pub fn id(&self) -> &SelectionId {
        &self.id
    }
}

impl Future for SelectionWait {
    type Output = SelectionOutcome;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Poll::Ready(outcome) = Pin::new(&mut this.rx).poll(cx) {
            this.settled = true;
            return Poll::Ready(outcome.unwrap_or(SelectionOutcome::Cancelled));
        }
        if Pin::new(&mut this.timeout).poll(cx).is_ready() {
            this.registry.expire(&this.id);
            this.settled = true;
            // expire() sent on the original oneshot; if this waiter still
            // holds rx, take the value or fall back to Expired.
            return Poll::Ready(match Pin::new(&mut this.rx).poll(cx) {
                Poll::Ready(Ok(outcome)) => outcome,
                Poll::Ready(Err(_)) | Poll::Pending => SelectionOutcome::Expired,
            });
        }
        Poll::Pending
    }
}

impl Drop for SelectionWait {
    fn drop(&mut self) {
        if !self.settled {
            self.registry.cancel(&self.id);
        }
    }
}

fn generate_id() -> Result<SelectionId, BeginSelectionError> {
    let mut bytes = [0_u8; 12];
    getrandom::fill(&mut bytes).map_err(|_| BeginSelectionError::EntropyUnavailable)?;
    SelectionId::new(format!("sel_{}", URL_SAFE_NO_PAD.encode(bytes)))
        .map_err(|_| BeginSelectionError::EntropyUnavailable)
}

fn validate_answer(spec: &PendingSelection, paths: &[String]) -> Result<(), ToolError> {
    if paths
        .iter()
        .any(|path| path.is_empty() || path.contains('\0'))
    {
        return Err(ToolError::MalformedSelection {
            message: "paths must be non-empty and must not contain NUL".to_owned(),
        });
    }
    match spec.kind {
        SelectionKind::Save => {
            if spec.multiple || paths.len() != 1 {
                return Err(ToolError::MalformedSelection {
                    message: "save accepts exactly one path".to_owned(),
                });
            }
            let path = Path::new(&paths[0]);
            if path.is_dir() {
                return Err(ToolError::MalformedSelection {
                    message: "save path must not be an existing directory".to_owned(),
                });
            }
            Ok(())
        }
        SelectionKind::Open => {
            if spec.multiple {
                if paths.is_empty() {
                    return Err(ToolError::MalformedSelection {
                        message: "multiple open requires at least one path".to_owned(),
                    });
                }
            } else if paths.len() != 1 {
                return Err(ToolError::MalformedSelection {
                    message: "single open accepts exactly one path".to_owned(),
                });
            }
            for path in paths {
                let meta = fs::metadata(path).map_err(|_| ToolError::MalformedSelection {
                    message: format!("open path {path:?} does not exist"),
                })?;
                if spec.directory {
                    if !meta.is_dir() {
                        return Err(ToolError::MalformedSelection {
                            message: format!("open directory path {path:?} is not a directory"),
                        });
                    }
                } else if !meta.is_file() {
                    return Err(ToolError::MalformedSelection {
                        message: format!("open file path {path:?} is not a file"),
                    });
                }
            }
            Ok(())
        }
    }
}

/// Maps a settled outcome onto the plugin-compatible picker result.
pub fn plugin_result(
    id: &SelectionId,
    multiple: bool,
    outcome: &SelectionOutcome,
) -> Result<serde_json::Value, ToolError> {
    match outcome {
        SelectionOutcome::Answered(paths) => {
            if multiple {
                Ok(serde_json::Value::Array(
                    paths
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ))
            } else {
                Ok(serde_json::Value::String(
                    paths.first().cloned().unwrap_or_default(),
                ))
            }
        }
        SelectionOutcome::Rejected => Ok(serde_json::Value::Null),
        SelectionOutcome::Expired => Err(ToolError::ExpiredSelection { id: id.clone() }),
        SelectionOutcome::Cancelled | SelectionOutcome::Shutdown => Err(ToolError::Unsupported {
            message: "selection was cancelled before settlement".to_owned(),
        }),
    }
}

/// Absolute path helper for tests and the host.
#[must_use]
pub fn path_string(path: impl AsRef<Path>) -> String {
    PathBuf::from(path.as_ref().as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    fn open_file_request() -> BeginSelectionRequest {
        BeginSelectionRequest {
            kind: SelectionKind::Open,
            directory: false,
            multiple: false,
            filters: Vec::new(),
            title: Some("Open".to_owned()),
            default_path: None,
            window: Some("main".to_owned()),
            webview: None,
        }
    }

    #[tokio::test]
    async fn answer_resolves_the_waiter_with_the_path() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("leaf.txt");
        File::create(&file).unwrap();
        let registry = SelectionRegistry::new("app:1");
        let wait = registry.begin(open_file_request()).unwrap();
        let id = wait.id().clone();
        assert_eq!(registry.pending().len(), 1);
        assert_eq!(registry.pending()[0].instance, "app:1");
        registry.answer(&id, &[path_string(&file)]).unwrap();
        let outcome = wait.await;
        assert_eq!(
            outcome,
            SelectionOutcome::Answered(vec![path_string(&file)])
        );
        assert!(registry.pending().is_empty());
        let again = registry.answer(&id, &[path_string(&file)]).unwrap_err();
        assert_eq!(again, ToolError::SettledSelection { id });
    }

    #[tokio::test]
    async fn reject_resolves_as_cancelled_plugin_null() {
        let registry = SelectionRegistry::new("app:1");
        let wait = registry.begin(open_file_request()).unwrap();
        let id = wait.id().clone();
        registry.reject(&id).unwrap();
        assert_eq!(wait.await, SelectionOutcome::Rejected);
        assert_eq!(
            plugin_result(&id, false, &SelectionOutcome::Rejected).unwrap(),
            serde_json::Value::Null
        );
    }

    #[tokio::test]
    async fn expiry_fails_typed_and_settles_once() {
        let registry = SelectionRegistry::with_ttl("app:1", Duration::from_millis(30));
        let wait = registry.begin(open_file_request()).unwrap();
        let id = wait.id().clone();
        assert_eq!(wait.await, SelectionOutcome::Expired);
        assert_eq!(
            registry.answer(&id, &["/nope".to_owned()]).unwrap_err(),
            ToolError::ExpiredSelection { id }
        );
    }

    #[tokio::test]
    async fn drop_cancels_so_the_caller_does_not_hang() {
        let registry = SelectionRegistry::new("app:1");
        let wait = registry.begin(open_file_request()).unwrap();
        let id = wait.id().clone();
        drop(wait);
        assert!(registry.pending().is_empty());
        assert_eq!(
            registry.reject(&id).unwrap_err(),
            ToolError::SettledSelection { id }
        );
    }

    #[tokio::test]
    async fn shutdown_settles_pending_waiters() {
        let registry = SelectionRegistry::new("app:1");
        let wait = registry.begin(open_file_request()).unwrap();
        registry.shutdown();
        assert_eq!(wait.await, SelectionOutcome::Shutdown);
        assert!(registry.pending().is_empty());
    }

    #[tokio::test]
    async fn unknown_id_and_wrong_instance_are_typed() {
        let a = SelectionRegistry::new("app:1");
        let b = SelectionRegistry::new("app:2");
        let wait = a.begin(open_file_request()).unwrap();
        let id = wait.id().clone();
        assert!(matches!(
            b.answer(&id, &["/nope".to_owned()]).unwrap_err(),
            ToolError::UnknownSelection { .. }
        ));
        drop(wait);
    }

    #[tokio::test]
    async fn save_rejects_multi_path_and_does_not_write() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("out.txt");
        let registry = SelectionRegistry::new("app:1");
        let wait = registry
            .begin(BeginSelectionRequest {
                kind: SelectionKind::Save,
                directory: false,
                multiple: false,
                filters: Vec::new(),
                title: None,
                default_path: None,
                window: None,
                webview: None,
            })
            .unwrap();
        let id = wait.id().clone();
        assert!(
            registry
                .answer(&id, &[path_string(&target), path_string(&target)])
                .is_err()
        );
        registry.answer(&id, &[path_string(&target)]).unwrap();
        assert!(!target.exists(), "save must not write the path");
        drop(wait);
    }

    #[tokio::test]
    async fn open_directory_rejects_a_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("leaf.txt");
        File::create(&file).unwrap();
        let registry = SelectionRegistry::new("app:1");
        let wait = registry
            .begin(BeginSelectionRequest {
                kind: SelectionKind::Open,
                directory: true,
                multiple: false,
                filters: Vec::new(),
                title: None,
                default_path: None,
                window: None,
                webview: None,
            })
            .unwrap();
        let id = wait.id().clone();
        let error = registry.answer(&id, &[path_string(&file)]).unwrap_err();
        assert!(matches!(error, ToolError::MalformedSelection { .. }));
        drop(wait);
    }

    #[tokio::test]
    async fn late_pending_read_sees_the_request() {
        let registry = SelectionRegistry::new("app:1");
        let wait = registry.begin(open_file_request()).unwrap();
        let body = registry.resource_body();
        assert_eq!(body.pending.len(), 1);
        assert_eq!(body.pending[0].id, *wait.id());
        drop(wait);
    }
}
