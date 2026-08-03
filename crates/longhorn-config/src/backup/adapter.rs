use std::{error::Error, fmt};

use longhorn_core::SchemaVersion;

use crate::{DomainDescriptor, Sha256Digest};

use super::{BackupExclusionReason, BackupLimits, BackupPayloadPath};

mod grouped;

pub use grouped::{
    BackupAdapterGroupedApplyKind, BackupAdapterGroupedApplyRequest, BackupAdapterGroupedRestore,
    BackupAdapterGroupedStageRequest, BackupAdapterGroupedVerifyRequest, BackupAdapterRestoreStage,
};

const MAX_ADAPTER_TEXT_BYTES: usize = 256;
const MAX_ADAPTER_RELATIVE_PATH_BYTES: usize = 384;

/// Stable description of an external transaction authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupAdapterConsistencyGroup {
    id: String,
    authority: String,
}

impl BackupAdapterConsistencyGroup {
    /// Validates an external consistency-group id and authority description.
    pub fn new(
        id: impl Into<String>,
        authority: impl Into<String>,
    ) -> Result<Self, BackupAdapterDeclarationError> {
        let id = id.into();
        validate_identifier("consistency group id", &id)?;
        let authority = authority.into();
        validate_text("transaction authority", &authority)?;
        Ok(Self { id, authority })
    }

    /// Returns the stable group id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the declared transaction authority.
    #[must_use]
    pub fn authority(&self) -> &str {
        &self.authority
    }
}

/// Capture authority declared by a custom adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupAdapterCaptureMode {
    /// Capture bounded bytes while Longhorn holds its store coordinator.
    CoordinatedBounded,
    /// Capture through an independent external transaction authority.
    ExternalSnapshot(BackupAdapterConsistencyGroup),
    /// Record an exclusion and never call the adapter.
    Excluded(BackupExclusionReason),
}

/// Restore guarantees declared by a custom adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupAdapterRestoreParticipation {
    /// Restore is unavailable and remains inspectable as excluded.
    Excluded(BackupExclusionReason),
    /// Restore is an explicit operation outside Longhorn's ordinary transaction.
    Separate,
    /// The adapter stages, journals, verifies, and can verify exact rollback.
    FailureAtomic,
    /// The adapter stages bounded target and rollback payloads for one
    /// Longhorn-owned multi-adapter transaction.
    GroupedFailureAtomic,
}

/// Complete capability declaration for one custom adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupAdapterCapabilities {
    capture: BackupAdapterCaptureMode,
    restore: BackupAdapterRestoreParticipation,
}

impl BackupAdapterCapabilities {
    /// Declares capture and restore participation.
    #[must_use]
    pub const fn new(
        capture: BackupAdapterCaptureMode,
        restore: BackupAdapterRestoreParticipation,
    ) -> Self {
        Self { capture, restore }
    }

    /// Returns capture authority.
    #[must_use]
    pub const fn capture(&self) -> &BackupAdapterCaptureMode {
        &self.capture
    }

    /// Returns restore guarantees.
    #[must_use]
    pub const fn restore(&self) -> &BackupAdapterRestoreParticipation {
        &self.restore
    }
}

/// Portable relative path contributed below one adapter-domain namespace.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BackupAdapterRelativePath(String);

impl BackupAdapterRelativePath {
    /// Validates a portable relative adapter payload path.
    pub fn new(value: impl Into<String>) -> Result<Self, BackupAdapterDeclarationError> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_ADAPTER_RELATIVE_PATH_BYTES {
            return Err(BackupAdapterDeclarationError::InvalidText {
                field: "adapter payload path",
                detail: format!("must contain 1 through {MAX_ADAPTER_RELATIVE_PATH_BYTES} bytes"),
            });
        }
        if value.contains('\\')
            || value.contains('\0')
            || value.split('/').any(|segment| {
                segment.is_empty()
                    || matches!(segment, "." | "..")
                    || !segment.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
            })
        {
            return Err(BackupAdapterDeclarationError::InvalidText {
                field: "adapter payload path",
                detail: "must be a portable relative path".into(),
            });
        }
        Ok(Self(value))
    }

    /// Returns the portable relative path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One immutable payload returned by a custom capture adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupAdapterPayload {
    relative_path: BackupAdapterRelativePath,
    pub(crate) bytes: Vec<u8>,
}

impl BackupAdapterPayload {
    /// Constructs one already-bounded candidate payload.
    #[must_use]
    pub fn new(relative_path: BackupAdapterRelativePath, bytes: Vec<u8>) -> Self {
        Self {
            relative_path,
            bytes,
        }
    }

    /// Returns the path below the adapter-domain namespace.
    #[must_use]
    pub fn relative_path(&self) -> &BackupAdapterRelativePath {
        &self.relative_path
    }

    /// Returns exact captured bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Complete source state returned by a custom capture adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupAdapterCapture {
    /// No persisted adapter state exists.
    Absent,
    /// Exact independently verified payloads exist.
    Present {
        /// Adapter-domain source schema.
        source_schema_version: SchemaVersion,
        /// One or more unique payloads.
        payloads: Vec<BackupAdapterPayload>,
    },
}

/// Bounded request supplied to custom capture.
#[derive(Clone, Copy, Debug)]
pub struct BackupAdapterCaptureRequest<'request> {
    descriptor: &'request DomainDescriptor,
    limits: BackupLimits,
}

impl<'request> BackupAdapterCaptureRequest<'request> {
    pub(crate) const fn new(descriptor: &'request DomainDescriptor, limits: BackupLimits) -> Self {
        Self { descriptor, limits }
    }

    /// Returns the schema-opaque registered descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &DomainDescriptor {
        self.descriptor
    }

    /// Returns Longhorn's complete capture bounds.
    #[must_use]
    pub const fn limits(&self) -> BackupLimits {
        self.limits
    }
}

/// Verified archive payload supplied back to an adapter.
#[derive(Clone, Copy, Debug)]
pub struct BackupAdapterPayloadRef<'payload> {
    path: &'payload BackupPayloadPath,
    bytes: &'payload [u8],
}

impl<'payload> BackupAdapterPayloadRef<'payload> {
    pub(crate) const fn new(path: &'payload BackupPayloadPath, bytes: &'payload [u8]) -> Self {
        Self { path, bytes }
    }

    /// Returns the complete confined archive path.
    #[must_use]
    pub const fn path(&self) -> &BackupPayloadPath {
        self.path
    }

    /// Returns exact verified payload bytes.
    #[must_use]
    pub const fn bytes(&self) -> &[u8] {
        self.bytes
    }
}

/// Side-effect-free custom restore inspection request.
#[derive(Clone, Debug)]
pub struct BackupAdapterInspectRequest<'request> {
    descriptor: &'request DomainDescriptor,
    source_schema_version: Option<SchemaVersion>,
    payloads: Vec<BackupAdapterPayloadRef<'request>>,
}

impl<'request> BackupAdapterInspectRequest<'request> {
    pub(crate) fn new(
        descriptor: &'request DomainDescriptor,
        source_schema_version: Option<SchemaVersion>,
        payloads: Vec<BackupAdapterPayloadRef<'request>>,
    ) -> Self {
        Self {
            descriptor,
            source_schema_version,
            payloads,
        }
    }

    /// Returns the schema-opaque registered descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &DomainDescriptor {
        self.descriptor
    }

    /// Returns the adapter-domain source schema.
    #[must_use]
    pub const fn source_schema_version(&self) -> Option<SchemaVersion> {
        self.source_schema_version
    }

    /// Returns every verified payload in manifest order.
    #[must_use]
    pub fn payloads(&self) -> &[BackupAdapterPayloadRef<'request>] {
        &self.payloads
    }
}

/// Adapter-produced semantic evidence retained by restore inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackupAdapterRestorePreview {
    target_evidence: Sha256Digest,
    current_evidence: Option<Sha256Digest>,
}

impl BackupAdapterRestorePreview {
    /// Records the staged semantic target and exact current semantic state.
    #[must_use]
    pub const fn new(
        target_evidence: Sha256Digest,
        current_evidence: Option<Sha256Digest>,
    ) -> Self {
        Self {
            target_evidence,
            current_evidence,
        }
    }

    /// Returns the adapter's staged semantic target digest.
    #[must_use]
    pub const fn target_evidence(&self) -> &Sha256Digest {
        &self.target_evidence
    }

    /// Returns current semantic evidence bound into confirmation.
    #[must_use]
    pub const fn current_evidence(&self) -> Option<&Sha256Digest> {
        self.current_evidence.as_ref()
    }
}

/// Explicit custom restore execution request.
#[derive(Clone, Debug)]
pub struct BackupAdapterRestoreRequest<'request> {
    inspect: BackupAdapterInspectRequest<'request>,
    preview: &'request BackupAdapterRestorePreview,
}

impl<'request> BackupAdapterRestoreRequest<'request> {
    pub(crate) const fn new(
        inspect: BackupAdapterInspectRequest<'request>,
        preview: &'request BackupAdapterRestorePreview,
    ) -> Self {
        Self { inspect, preview }
    }

    /// Returns the verified archive request.
    #[must_use]
    pub const fn inspect(&self) -> &BackupAdapterInspectRequest<'request> {
        &self.inspect
    }

    /// Returns the confirmation-bound inspected target.
    #[must_use]
    pub const fn preview(&self) -> &BackupAdapterRestorePreview {
        self.preview
    }
}

/// Terminal state reported by the adapter's transaction authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupAdapterRestoreOutcome {
    /// Target publication and verification succeeded.
    Verified {
        /// Exact semantic evidence after publication.
        evidence: Sha256Digest,
    },
    /// Publication failed and exact prior state was restored and verified.
    RolledBack {
        /// Exact semantic evidence after rollback.
        evidence: Sha256Digest,
    },
    /// The adapter retained its journal and blocks normal mutation.
    RecoveryRequired,
}

/// One schema-opaque custom backup and restore adapter.
///
/// Declaring [`BackupAdapterRestoreParticipation::FailureAtomic`] is a
/// contract that `restore` stages target state, preserves exact current state,
/// durably journals before publication, verifies publication, and verifies
/// rollback before returning `RolledBack`. Longhorn reports but does not take
/// over the adapter's external transaction authority.
pub trait BackupAdapter {
    /// Returns the stable adapter id.
    fn id(&self) -> &super::BackupAdapterId;

    /// Returns immutable capture and restore capabilities.
    fn capabilities(&self) -> &BackupAdapterCapabilities;

    /// Captures exact state through the declared authority.
    fn capture(
        &self,
        request: BackupAdapterCaptureRequest<'_>,
    ) -> Result<BackupAdapterCapture, BackupAdapterError>;

    /// Inspects verified archive payloads without mutation.
    fn inspect(
        &self,
        request: BackupAdapterInspectRequest<'_>,
    ) -> Result<BackupAdapterRestorePreview, BackupAdapterError>;

    /// Executes one explicit adapter-owned restore transaction.
    fn restore(
        &self,
        request: BackupAdapterRestoreRequest<'_>,
    ) -> Result<BackupAdapterRestoreOutcome, BackupAdapterError>;

    /// Returns the optional grouped transaction extension.
    ///
    /// Implementations declaring
    /// [`BackupAdapterRestoreParticipation::GroupedFailureAtomic`] must return
    /// an extension. Other participation modes return `None`.
    fn grouped_restore(&self) -> Option<&dyn BackupAdapterGroupedRestore> {
        None
    }
}

/// Stable bounded adapter failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupAdapterError {
    /// The declared external authority is currently unavailable.
    Unavailable,
    /// The adapter failed with a stable non-secret code.
    Failed(String),
}

impl BackupAdapterError {
    /// Validates a stable failure code.
    pub fn failed(code: impl Into<String>) -> Result<Self, BackupAdapterDeclarationError> {
        let code = code.into();
        validate_identifier("adapter failure code", &code)?;
        Ok(Self::Failed(code))
    }
}

impl fmt::Display for BackupAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("adapter authority is unavailable"),
            Self::Failed(code) => write!(formatter, "adapter failed: {code}"),
        }
    }
}

impl Error for BackupAdapterError {}

/// Invalid adapter capability, path, or stable metadata declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackupAdapterDeclarationError {
    /// A bounded field was empty, malformed, or too long.
    InvalidText {
        /// Invalid field.
        field: &'static str,
        /// Stable validation detail.
        detail: String,
    },
}

impl fmt::Display for BackupAdapterDeclarationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidText { field, detail } => {
                write!(formatter, "invalid backup {field}: {detail}")
            }
        }
    }
}

impl Error for BackupAdapterDeclarationError {}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), BackupAdapterDeclarationError> {
    validate_text(field, value)?;
    if value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
    }) {
        Ok(())
    } else {
        Err(BackupAdapterDeclarationError::InvalidText {
            field,
            detail: "must contain lowercase ASCII letters, digits, '.', '_', or '-'".into(),
        })
    }
}

fn validate_text(field: &'static str, value: &str) -> Result<(), BackupAdapterDeclarationError> {
    if value.is_empty() || value.len() > MAX_ADAPTER_TEXT_BYTES {
        return Err(BackupAdapterDeclarationError::InvalidText {
            field,
            detail: format!("must contain 1 through {MAX_ADAPTER_TEXT_BYTES} bytes"),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(BackupAdapterDeclarationError::InvalidText {
            field,
            detail: "cannot contain control characters".into(),
        });
    }
    Ok(())
}
