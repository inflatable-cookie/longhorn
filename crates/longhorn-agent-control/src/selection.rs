//! Per-instance pending-selection registry (contract 022).
//!
//! One registry belongs to one control-surface instance. Agent-originated
//! `open`/`save` calls begin a request here; MCP `answer_selection` /
//! `reject_selection` settle it exactly once. Resource reads return the
//! still-pending set so a late subscriber can discover a request created
//! before it joined. Expiry, cancellation, and shutdown settle the waiter
//! so the JS caller is never left awaiting indefinitely.

use std::{
    fmt, fs,
    future::Future,
    path::Path,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use indexmap::IndexMap;
use longhorn_core::OpaqueIdError;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{oneshot, watch},
    time::Instant,
};

use crate::{SelectionId, ToolError};

/// Bound an unanswered request lives before it expires with a typed error.
pub const DEFAULT_SELECTION_TTL: Duration = Duration::from_secs(60);

/// How long a settled or expired tombstone is retained so a late MCP answer
/// still observes `SettledSelection` or `ExpiredSelection` instead of
/// `UnknownSelection`.
const TOMBSTONE_RETENTION: Duration = Duration::from_secs(300);

/// Hard cap on retained tombstones so rapid churn cannot grow memory without
/// bound; oldest tombstones are evicted first once exceeded.
const MAX_TOMBSTONES: usize = 256;

/// Dialog filter copied from the caller's picker options.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SelectionFilter {
    /// Filter display name.
    pub name: String,
    /// File extensions without a leading dot.
    pub extensions: Vec<String>,
}

/// Open vs save. Directory intent is [`BeginSelectionRequest::directory`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum SelectionKind {
    /// Tauri JS `open`.
    Open,
    /// Tauri JS `save`. Longhorn never writes the chosen path.
    Save,
}

/// Renderer options accepted by the begin-selection command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BeginSelectionArgs {
    /// Open or save.
    pub kind: SelectionKind,
    /// Directory intent; only meaningful for open.
    #[serde(default)]
    pub directory: bool,
    /// Multiple paths; only meaningful for open.
    #[serde(default)]
    pub multiple: bool,
    /// Optional extension filters. Hints for the agent; not enforced.
    #[serde(default)]
    pub filters: Vec<SelectionFilter>,
    /// Dialog title, when the caller supplied one.
    #[cfg_attr(feature = "bindings", ts(optional = nullable))]
    pub title: Option<String>,
    /// Default-path hint, never an answer.
    #[cfg_attr(feature = "bindings", ts(optional = nullable))]
    pub default_path: Option<String>,
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
    Rejected {
        /// Agent-supplied reason, available to the host waiter only.
        reason: String,
    },
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
    EntropyUnavailable(getrandom::Error),
    /// The generated id did not satisfy the opaque-id contract.
    InvalidId(OpaqueIdError),
    /// Save accepts a single file path.
    InvalidSaveOptions,
}

impl fmt::Display for BeginSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntropyUnavailable(_) => formatter.write_str("selection id entropy unavailable"),
            Self::InvalidId(source) => {
                write!(formatter, "generated selection id is invalid: {source}")
            }
            Self::InvalidSaveOptions => {
                formatter.write_str("save does not accept multiple or directory")
            }
        }
    }
}

impl std::error::Error for BeginSelectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EntropyUnavailable(source) => Some(source),
            Self::InvalidId(source) => Some(source),
            Self::InvalidSaveOptions => None,
        }
    }
}

impl From<BeginSelectionError> for ToolError {
    fn from(error: BeginSelectionError) -> Self {
        let message = error.to_string();
        match error {
            BeginSelectionError::InvalidSaveOptions => Self::InvalidArgument { message },
            _ => Self::HostFailure { message },
        }
    }
}

struct PendingSlot {
    spec: PendingSelection,
    tx: oneshot::Sender<SelectionOutcome>,
    deadline: Instant,
}

enum SlotState {
    Pending(PendingSlot),
    Settled { settled_at: Instant },
    Expired { settled_at: Instant },
}

struct Inner {
    instance: String,
    ttl: Duration,
    generation: AtomicU64,
    generation_tx: watch::Sender<u64>,
    slots: Mutex<IndexMap<SelectionId, SlotState>>,
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
        let (generation_tx, _rx) = watch::channel(0_u64);
        Self {
            inner: Arc::new(Inner {
                instance: instance.into(),
                ttl,
                generation: AtomicU64::new(0),
                generation_tx,
                slots: Mutex::new(IndexMap::new()),
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

    /// Subscribes to generation changes (push channel for MCP `listen`).
    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.inner.generation_tx.subscribe()
    }

    /// Still-pending requests in publication order, newest last.
    #[must_use]
    pub fn pending(&self) -> Vec<PendingSelection> {
        self.sweep();
        let slots = self.inner.slots.lock().expect("selection registry mutex");
        slots
            .values()
            .filter_map(|slot| match slot {
                SlotState::Pending(pending) => Some(pending.spec.clone()),
                SlotState::Settled { .. } | SlotState::Expired { .. } => None,
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
    ///
    /// Needs no Tokio runtime: the TTL timer is created on the waiter's first
    /// poll, so awaiting it requires a runtime with timers enabled. If that
    /// poll panics, the waiter is dropped during unwinding and cancels its
    /// pending entry.
    pub fn begin(
        &self,
        request: BeginSelectionRequest,
    ) -> Result<SelectionWait, BeginSelectionError> {
        if request.kind == SelectionKind::Save && (request.multiple || request.directory) {
            return Err(BeginSelectionError::InvalidSaveOptions);
        }
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
            slots.insert(
                id.clone(),
                SlotState::Pending(PendingSlot { spec, tx, deadline }),
            );
            self.bump();
        }
        Ok(SelectionWait {
            id: id.clone(),
            registry: self.clone(),
            deadline,
            timeout: None,
            rx,
            settled: false,
        })
    }

    /// Agent answer: validate cardinality and path kind, then settle once.
    pub fn answer(&self, id: &SelectionId, paths: &[String]) -> Result<(), ToolError> {
        self.answer_with(id, paths, validate_answer)
    }

    fn answer_with(
        &self,
        id: &SelectionId,
        paths: &[String],
        validate: impl FnOnce(SelectionKind, bool, bool, &[String]) -> Result<(), ToolError>,
    ) -> Result<(), ToolError> {
        let (kind, directory, multiple) = {
            let mut slots = self.inner.slots.lock().expect("selection registry mutex");
            self.sweep_locked(&mut slots, Instant::now());
            let slot = slots
                .get(id)
                .ok_or_else(|| ToolError::UnknownSelection { id: id.clone() })?;
            let spec = &slot.pending(id)?.spec;
            (spec.kind, spec.directory, spec.multiple)
        };

        let outcome = validate(kind, directory, multiple, paths)
            .map(|()| SelectionOutcome::Answered(paths.to_vec()));
        // Recheck after filesystem I/O for concurrent settlement or expiry.
        self.settle_terminal(id, outcome)
    }

    /// Explicit reject: the host receives the reason and the JS call resolves as null.
    pub fn reject(&self, id: &SelectionId, reason: String) -> Result<(), ToolError> {
        self.settle_terminal(id, Ok(SelectionOutcome::Rejected { reason }))
    }

    /// Cancel a still-pending waiter (host drop, not an MCP reject).
    pub fn cancel(&self, id: &SelectionId) {
        let _ = self.settle_terminal(id, Ok(SelectionOutcome::Cancelled));
    }

    /// Settle every pending request. Idempotent.
    pub fn shutdown(&self) {
        let now = Instant::now();
        let mut slots = self.inner.slots.lock().expect("selection registry mutex");
        let mut bumped = false;
        for slot in slots.values_mut() {
            if matches!(slot, SlotState::Pending(_))
                && let SlotState::Pending(PendingSlot { tx, .. }) =
                    std::mem::replace(slot, SlotState::Settled { settled_at: now })
            {
                let _ = tx.send(SelectionOutcome::Shutdown);
                bumped = true;
            }
        }
        prune_tombstones(&mut slots, now);
        if bumped {
            self.bump();
        }
    }

    fn settle_terminal(
        &self,
        id: &SelectionId,
        outcome: Result<SelectionOutcome, ToolError>,
    ) -> Result<(), ToolError> {
        let mut slots = self.inner.slots.lock().expect("selection registry mutex");
        let now = Instant::now();
        self.sweep_locked(&mut slots, now);
        let slot = slots
            .get_mut(id)
            .ok_or_else(|| ToolError::UnknownSelection { id: id.clone() })?;
        slot.pending(id)?;
        let outcome = outcome?;
        if let SlotState::Pending(PendingSlot { tx, .. }) =
            std::mem::replace(slot, SlotState::Settled { settled_at: now })
        {
            let _ = tx.send(outcome);
        }
        prune_tombstones(&mut slots, now);
        self.bump();
        Ok(())
    }

    fn sweep(&self) {
        let mut slots = self.inner.slots.lock().expect("selection registry mutex");
        self.sweep_locked(&mut slots, Instant::now());
    }

    fn sweep_locked(&self, slots: &mut IndexMap<SelectionId, SlotState>, now: Instant) {
        let mut bumped = false;
        for slot in slots.values_mut() {
            if matches!(slot, SlotState::Pending(PendingSlot { deadline, .. }) if *deadline <= now)
                && let SlotState::Pending(PendingSlot { tx, .. }) =
                    std::mem::replace(slot, SlotState::Expired { settled_at: now })
            {
                let _ = tx.send(SelectionOutcome::Expired);
                bumped = true;
            }
        }
        prune_tombstones(slots, now);
        if bumped {
            self.bump();
        }
    }

    fn bump(&self) {
        let next = self.inner.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.inner.generation_tx.send(next);
    }
}

impl SlotState {
    fn pending(&self, id: &SelectionId) -> Result<&PendingSlot, ToolError> {
        match self {
            Self::Pending(slot) => Ok(slot),
            Self::Settled { .. } => Err(ToolError::SettledSelection { id: id.clone() }),
            Self::Expired { .. } => Err(ToolError::ExpiredSelection { id: id.clone() }),
        }
    }
}

/// Removes tombstones older than [`TOMBSTONE_RETENTION`] and evicts the oldest
/// tombstones when more than [`MAX_TOMBSTONES`] remain.
fn prune_tombstones(slots: &mut IndexMap<SelectionId, SlotState>, now: Instant) {
    slots.retain(|_, slot| match slot {
        SlotState::Pending(_) => true,
        SlotState::Settled { settled_at } | SlotState::Expired { settled_at } => {
            now.saturating_duration_since(*settled_at) < TOMBSTONE_RETENTION
        }
    });
    let mut tombstones: Vec<(SelectionId, Instant)> = slots
        .iter()
        .filter_map(|(id, slot)| match slot {
            SlotState::Settled { settled_at } | SlotState::Expired { settled_at } => {
                Some((id.clone(), *settled_at))
            }
            SlotState::Pending(_) => None,
        })
        .collect();
    if tombstones.len() > MAX_TOMBSTONES {
        tombstones.sort_by_key(|(_, settled_at)| *settled_at);
        let excess = tombstones.len() - MAX_TOMBSTONES;
        for (id, _) in tombstones.into_iter().take(excess) {
            slots.shift_remove(&id);
        }
    }
}

/// Waiter the JS/host caller holds until the request settles.
///
/// Dropping without awaiting cancels the request. Awaiting past the TTL
/// expires it even if the agent never answers.
pub struct SelectionWait {
    id: SelectionId,
    registry: SelectionRegistry,
    deadline: Instant,
    /// Created on the first poll, so `begin` never touches the Tokio timer.
    timeout: Option<Pin<Box<tokio::time::Sleep>>>,
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
        let deadline = this.deadline;
        let timeout = this
            .timeout
            .get_or_insert_with(|| Box::pin(tokio::time::sleep_until(deadline)));
        if Pin::new(timeout).poll(cx).is_ready() {
            // Single expiry path on the shared Tokio clock: sweep() atomically
            // settles past-deadline slots and sends the authoritative outcome
            // on `tx` while holding the lock.
            this.registry.sweep();
            if let Poll::Ready(outcome) = Pin::new(&mut this.rx).poll(cx) {
                this.settled = true;
                return Poll::Ready(outcome.unwrap_or(SelectionOutcome::Cancelled));
            }
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
    getrandom::fill(&mut bytes).map_err(BeginSelectionError::EntropyUnavailable)?;
    SelectionId::new(format!("sel_{}", URL_SAFE_NO_PAD.encode(bytes)))
        .map_err(BeginSelectionError::InvalidId)
}

fn validate_answer(
    kind: SelectionKind,
    directory: bool,
    multiple: bool,
    paths: &[String],
) -> Result<(), ToolError> {
    if paths
        .iter()
        .any(|path| path.is_empty() || path.contains('\0'))
    {
        return Err(ToolError::MalformedSelection {
            message: "paths must be non-empty and must not contain NUL".to_owned(),
        });
    }
    if paths.iter().any(|path| !Path::new(path).is_absolute()) {
        return Err(ToolError::MalformedSelection {
            message: "answer paths must be absolute".to_owned(),
        });
    }
    match kind {
        SelectionKind::Save => {
            if multiple || paths.len() != 1 {
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
            if multiple {
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
                if directory {
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
        SelectionOutcome::Rejected { .. } => Ok(serde_json::Value::Null),
        SelectionOutcome::Expired => Err(ToolError::ExpiredSelection { id: id.clone() }),
        SelectionOutcome::Cancelled | SelectionOutcome::Shutdown => {
            Err(ToolError::CancelledSelection { id: id.clone() })
        }
    }
}

/// Converts a path to a lossy UTF-8 string for tests and the host.
///
/// Does not canonicalize or absolutize the path.
#[must_use]
pub fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
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

    #[test]
    fn invalid_save_options_never_publish_a_request() {
        let registry = SelectionRegistry::new("app:1");
        for (multiple, directory) in [(true, false), (false, true), (true, true)] {
            let result = registry.begin(BeginSelectionRequest {
                kind: SelectionKind::Save,
                directory,
                multiple,
                ..open_file_request()
            });
            assert!(matches!(
                result,
                Err(BeginSelectionError::InvalidSaveOptions)
            ));
        }
        assert!(registry.pending().is_empty());
        assert_eq!(registry.generation(), 0);
    }

    #[test]
    fn an_id_error_is_distinct_from_entropy_failure_and_keeps_its_source() {
        use std::error::Error as _;

        let id_error = SelectionId::new("")
            .map_err(BeginSelectionError::InvalidId)
            .unwrap_err();
        assert!(matches!(id_error, BeginSelectionError::InvalidId(_)));
        assert_eq!(
            id_error.source().unwrap().downcast_ref::<OpaqueIdError>(),
            Some(&OpaqueIdError::Empty)
        );
        let entropy_error = BeginSelectionError::EntropyUnavailable(getrandom::Error::UNSUPPORTED);
        assert_eq!(
            entropy_error
                .source()
                .unwrap()
                .downcast_ref::<getrandom::Error>(),
            Some(&getrandom::Error::UNSUPPORTED)
        );
    }

    #[test]
    fn relative_answers_leave_the_selection_pending() {
        let registry = SelectionRegistry::new("app:1");
        for kind in [SelectionKind::Open, SelectionKind::Save] {
            let wait = registry
                .begin(BeginSelectionRequest {
                    kind,
                    ..open_file_request()
                })
                .unwrap();
            assert_eq!(
                registry.answer(wait.id(), &["Cargo.toml".to_owned()]),
                Err(ToolError::MalformedSelection {
                    message: "answer paths must be absolute".to_owned(),
                })
            );
            assert_eq!(registry.pending().len(), 1);
        }
    }

    #[test]
    fn pending_is_in_publication_order_after_settlement() {
        let registry = SelectionRegistry::new("app:1");
        let waits: Vec<_> = (0..16)
            .map(|_| registry.begin(open_file_request()).unwrap())
            .collect();
        registry.cancel(waits[3].id());
        let expected: Vec<_> = waits
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != 3)
            .map(|(_, wait)| wait.id().clone())
            .collect();
        assert_eq!(
            registry
                .pending()
                .into_iter()
                .map(|spec| spec.id)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[tokio::test]
    async fn validation_releases_the_lock_and_cannot_overwrite_a_concurrent_reject() {
        let registry = SelectionRegistry::new("app:1");
        let wait = registry.begin(open_file_request()).unwrap();
        let id = wait.id().clone();
        let result = registry.answer_with(&id, &["report.txt".to_owned()], |_, _, _, _| {
            assert!(registry.inner.slots.try_lock().is_ok());
            registry.reject(&id, "cancelled".to_owned())?;
            Ok(())
        });
        assert_eq!(result, Err(ToolError::SettledSelection { id }));
        assert!(matches!(wait.await, SelectionOutcome::Rejected { .. }));
    }

    #[tokio::test]
    async fn answer_expiring_during_validation_cannot_settle_as_answered() {
        let registry = SelectionRegistry::new("app:1");
        let wait = registry.begin(open_file_request()).unwrap();
        let id = wait.id().clone();
        let result = registry.answer_with(&id, &["report.txt".to_owned()], |_, _, _, _| {
            let mut slots = registry.inner.slots.lock().unwrap();
            let Some(SlotState::Pending(slot)) = slots.get_mut(&id) else {
                panic!("expected a pending request");
            };
            slot.deadline = Instant::now();
            Ok(())
        });
        assert_eq!(result, Err(ToolError::ExpiredSelection { id }));
        assert_eq!(wait.await, SelectionOutcome::Expired);
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
        registry
            .reject(&id, "cancelled by test".to_owned())
            .unwrap();
        let outcome = wait.await;
        assert_eq!(
            outcome,
            SelectionOutcome::Rejected {
                reason: "cancelled by test".to_owned()
            }
        );
        assert_eq!(
            plugin_result(&id, false, &outcome).unwrap(),
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
            registry
                .reject(&id, "cancelled by test".to_owned())
                .unwrap_err(),
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

    #[tokio::test]
    async fn cancelled_and_shutdown_outcomes_map_to_cancelled_selection() {
        let id = SelectionId::new("sel_test").unwrap();
        assert_eq!(
            plugin_result(&id, false, &SelectionOutcome::Cancelled).unwrap_err(),
            ToolError::CancelledSelection { id: id.clone() }
        );
        assert_eq!(
            plugin_result(&id, false, &SelectionOutcome::Shutdown).unwrap_err(),
            ToolError::CancelledSelection { id }
        );
    }

    #[tokio::test]
    async fn tombstones_are_bounded_by_max_tombstones() {
        let registry = SelectionRegistry::new("app:1");
        let mut ids = Vec::new();
        for _ in 0..(MAX_TOMBSTONES + 16) {
            let wait = registry.begin(open_file_request()).unwrap();
            let id = wait.id().clone();
            registry
                .reject(&id, "cancelled by test".to_owned())
                .unwrap();
            let _ = wait.await;
            ids.push(id);
        }
        let slot_count = registry
            .inner
            .slots
            .lock()
            .expect("selection registry mutex")
            .len();
        assert!(
            slot_count <= MAX_TOMBSTONES,
            "tombstones must stay bounded at {MAX_TOMBSTONES}, got {slot_count}"
        );
        // Most recent tombstone is still remembered.
        let latest = ids.last().unwrap();
        assert_eq!(
            registry
                .reject(latest, "cancelled by test".to_owned())
                .unwrap_err(),
            ToolError::SettledSelection { id: latest.clone() }
        );
    }

    #[tokio::test]
    async fn expiry_settles_registry_and_waiter_atomically_on_shared_clock() {
        let registry = SelectionRegistry::with_ttl("app:1", Duration::from_millis(25));
        let wait = registry.begin(open_file_request()).unwrap();
        let id = wait.id().clone();
        assert_eq!(wait.await, SelectionOutcome::Expired);
        assert!(registry.pending().is_empty());
        assert_eq!(
            registry.answer(&id, &["/nope".to_owned()]).unwrap_err(),
            ToolError::ExpiredSelection { id }
        );
    }

    #[test]
    fn begin_needs_no_tokio_runtime() {
        let registry = SelectionRegistry::new("app:1");
        let wait = registry.begin(open_file_request()).unwrap();
        assert_eq!(registry.pending().len(), 1);
        drop(wait);
        assert!(registry.pending().is_empty());
    }

    #[test]
    fn waiter_panicking_on_a_timerless_runtime_leaves_nothing_pending() {
        let registry = SelectionRegistry::new("app:1");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.block_on(async {
                let wait = registry.begin(open_file_request()).unwrap();
                wait.await
            })
        }));
        assert!(outcome.is_err(), "awaiting without timers must fail loudly");
        assert!(
            registry.pending().is_empty(),
            "a waiter dropped by that panic must not leave a pending entry"
        );
    }
}
