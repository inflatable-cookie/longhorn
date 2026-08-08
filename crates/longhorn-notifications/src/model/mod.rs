//! Notification domain model types.

mod draft;
mod projection;
mod record;
mod request;

pub use draft::{
    NotificationAction, NotificationDraft, NotificationReadState, NotificationRetentionClass,
    NotificationSeverity,
};
pub use projection::{NotificationLedgerProjection, NotificationPage};
pub use record::{NotificationAuthorityCursor, NotificationRecord};
pub use request::{
    NotificationAdd, NotificationClear, NotificationClearTarget, NotificationPublishOnce,
    NotificationPublishOutcome, NotificationReplace, NotificationRetentionChange, NotificationSeen,
};
