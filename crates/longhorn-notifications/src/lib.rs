//! Pure finite retained notification ledger.
//!
//! Retained truth is independent from transient toast presentation. Consumers
//! own product policy, action execution, persistence, and authorization.

mod error;
mod identity;
mod ledger;
mod limits;
mod model;
mod protocol;
mod retention;

#[cfg(feature = "operation")]
mod operation_observer;

pub use error::NotificationLedgerError;
pub use identity::{
    NotificationAuthorityEpoch, NotificationAuthorityEpochError, NotificationSequence,
    NotificationSequenceOverflow,
};
pub use ledger::NotificationLedger;
pub use limits::{
    MAXIMUM_NOTIFICATION_ACTION_LABEL_BYTES, MAXIMUM_NOTIFICATION_ACTIONS,
    MAXIMUM_NOTIFICATION_ENCODED_WEIGHT, MAXIMUM_NOTIFICATION_PAGE_SIZE,
    MAXIMUM_NOTIFICATION_SUMMARY_BYTES, MAXIMUM_NOTIFICATION_TITLE_BYTES,
    MAXIMUM_RETAINED_NOTIFICATIONS, NotificationActionLabel, NotificationActionLabelError,
    NotificationLedgerLimits, NotificationLedgerLimitsError, NotificationPageSizeError,
    NotificationSummary, NotificationSummaryError, NotificationTitle, NotificationTitleError,
};
pub use model::{
    NotificationAction, NotificationAdd, NotificationAuthorityCursor, NotificationClear,
    NotificationClearTarget, NotificationDraft, NotificationLedgerProjection, NotificationPage,
    NotificationPublishOnce, NotificationPublishOutcome, NotificationReadState, NotificationRecord,
    NotificationReplace, NotificationRetentionChange, NotificationRetentionClass, NotificationSeen,
    NotificationSeverity,
};
#[cfg(feature = "operation")]
pub use operation_observer::{
    OperationNotificationObservationError, OperationNotificationPolicy,
    OperationNotificationPublication, observe_terminal_operation, publish_operation_notification,
};
pub use protocol::*;
pub use retention::{
    NotificationMutationReceipt, NotificationRemoval, NotificationRemovalReason,
    NotificationRemovalReceipt,
};
