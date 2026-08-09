//! Draft notification content and actions.

use longhorn_core::{
    NotificationActionReferenceId, NotificationCauseId, NotificationProducerToken,
    NotificationReplacementKey, NotificationSourceId,
};

use crate::{
    MAXIMUM_NOTIFICATION_ACTIONS, NotificationActionLabel, NotificationSummary, NotificationTitle,
};

/// Closed, product-neutral notification severity.
/// Closed, product-neutral notification severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationSeverity {
    /// Routine information.
    Info,
    /// Successful completion.
    Success,
    /// Recoverable concern.
    Warning,
    /// Failed work or degraded behavior.
    Error,
    /// Urgent failure requiring attention.
    Critical,
}

impl NotificationSeverity {
    /// Every severity, in ascending order of loudness.
    ///
    /// Ordered rather than alphabetical, and exhaustive by construction: the
    /// generated TypeScript label map is built from this, so a new severity
    /// that is not added here fails the bindings gate rather than rendering
    /// blank in a webview.
    pub const ALL: [Self; 5] = [
        Self::Info,
        Self::Success,
        Self::Warning,
        Self::Error,
        Self::Critical,
    ];

    /// The wire name, which is also the generated map's key.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }

    /// The operator-facing name.
    ///
    /// Lives here rather than in a projection because what a severity is
    /// called is a property of the severity. Two surfaces that each invented
    /// their own wording is exactly the drift memo 022 recorded.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "Information",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Critical => "Critical",
        }
    }
}

/// Explicit read state independent from retention and presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationReadState {
    /// The user has not seen the retained record.
    Unseen,
    /// The record was explicitly marked seen.
    Seen,
}

/// Consumer-selected retention treatment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationRetentionClass {
    /// Oldest records may be pruned to satisfy limits.
    Standard,
    /// Automatic pruning must not remove the record.
    Protected,
}

/// Bounded semantic action data. It contains no executable callback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationAction {
    pub(crate) reference_id: NotificationActionReferenceId,
    pub(crate) label: NotificationActionLabel,
}

impl NotificationAction {
    /// Constructs a semantic action reference.
    #[must_use]
    pub const fn new(
        reference_id: NotificationActionReferenceId,
        label: NotificationActionLabel,
    ) -> Self {
        Self {
            reference_id,
            label,
        }
    }

    /// Returns the consumer-owned semantic action reference.
    #[must_use]
    pub const fn reference_id(&self) -> &NotificationActionReferenceId {
        &self.reference_id
    }

    /// Returns the presentation label.
    #[must_use]
    pub const fn label(&self) -> &NotificationActionLabel {
        &self.label
    }
}

/// Consumer-supplied bounded notification metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationDraft {
    pub(crate) source_id: NotificationSourceId,
    pub(crate) severity: NotificationSeverity,
    pub(crate) title: NotificationTitle,
    pub(crate) summary: NotificationSummary,
    pub(crate) cause_id: Option<NotificationCauseId>,
    pub(crate) actions: Vec<NotificationAction>,
    pub(crate) replacement_key: Option<NotificationReplacementKey>,
    pub(crate) producer_token: Option<NotificationProducerToken>,
    pub(crate) retention_class: NotificationRetentionClass,
    pub(crate) presentation_time_unix_ms: Option<i64>,
}

impl NotificationDraft {
    /// Constructs the required record metadata with standard retention.
    #[must_use]
    pub const fn new(
        source_id: NotificationSourceId,
        severity: NotificationSeverity,
        title: NotificationTitle,
        summary: NotificationSummary,
    ) -> Self {
        Self {
            source_id,
            severity,
            title,
            summary,
            cause_id: None,
            actions: Vec::new(),
            replacement_key: None,
            producer_token: None,
            retention_class: NotificationRetentionClass::Standard,
            presentation_time_unix_ms: None,
        }
    }

    /// Attaches an opaque cause reference.
    #[must_use]
    pub fn with_cause(mut self, cause_id: NotificationCauseId) -> Self {
        self.cause_id = Some(cause_id);
        self
    }

    /// Attaches bounded semantic action references.
    pub fn with_actions(
        mut self,
        actions: Vec<NotificationAction>,
    ) -> Result<Self, crate::NotificationLedgerError> {
        if actions.len() > MAXIMUM_NOTIFICATION_ACTIONS {
            return Err(crate::NotificationLedgerError::TooManyActions {
                maximum: MAXIMUM_NOTIFICATION_ACTIONS,
                actual: actions.len(),
            });
        }
        self.actions = actions;
        Ok(self)
    }

    /// Selects an explicit replacement key.
    #[must_use]
    pub fn with_replacement_key(mut self, key: NotificationReplacementKey) -> Self {
        self.replacement_key = Some(key);
        self
    }

    /// Selects a durable idempotent producer token.
    #[must_use]
    pub fn with_producer_token(mut self, token: NotificationProducerToken) -> Self {
        self.producer_token = Some(token);
        self
    }

    /// Selects consumer-owned retention treatment.
    #[must_use]
    pub const fn with_retention_class(mut self, class: NotificationRetentionClass) -> Self {
        self.retention_class = class;
        self
    }

    /// Attaches an injected wall-clock presentation time.
    #[must_use]
    pub const fn with_presentation_time_unix_ms(mut self, time: i64) -> Self {
        self.presentation_time_unix_ms = Some(time);
        self
    }

    /// Returns the source identity.
    #[must_use]
    pub const fn source_id(&self) -> &NotificationSourceId {
        &self.source_id
    }

    /// Returns severity.
    #[must_use]
    pub const fn severity(&self) -> NotificationSeverity {
        self.severity
    }

    /// Returns title.
    #[must_use]
    pub const fn title(&self) -> &NotificationTitle {
        &self.title
    }

    /// Returns summary.
    #[must_use]
    pub const fn summary(&self) -> &NotificationSummary {
        &self.summary
    }

    /// Returns the optional cause reference.
    #[must_use]
    pub const fn cause_id(&self) -> Option<&NotificationCauseId> {
        self.cause_id.as_ref()
    }

    /// Returns semantic actions.
    #[must_use]
    pub fn actions(&self) -> &[NotificationAction] {
        &self.actions
    }

    /// Returns the explicit replacement key.
    #[must_use]
    pub const fn replacement_key(&self) -> Option<&NotificationReplacementKey> {
        self.replacement_key.as_ref()
    }

    /// Returns the idempotent producer token.
    #[must_use]
    pub const fn producer_token(&self) -> Option<&NotificationProducerToken> {
        self.producer_token.as_ref()
    }

    /// Returns retention treatment.
    #[must_use]
    pub const fn retention_class(&self) -> NotificationRetentionClass {
        self.retention_class
    }

    /// Returns the optional injected wall-clock time.
    #[must_use]
    pub const fn presentation_time_unix_ms(&self) -> Option<i64> {
        self.presentation_time_unix_ms
    }
}
