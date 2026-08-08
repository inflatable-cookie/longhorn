//! Strict product-neutral renderer and transport protocol.

mod error;
mod event;
mod execute;
mod mutation;
mod projection;
mod snapshot;
mod version;

pub use error::NotificationProtocolError;
pub(crate) use error::{NotificationProtocolInputError, incompatible_rejection, project_usize};
pub use event::{NotificationChangedEvent, NotificationChangedKind};
pub use mutation::{
    NotificationMutationCommand, NotificationMutationReceiptProjection, NotificationMutationResult,
    NotificationRejection, NotificationRejectionCode, NotificationRemovalProjection,
    NotificationRemovalReasonProjection,
};
pub use projection::{
    NotificationActionProjection, NotificationAuthorityProjection, NotificationDraftProjection,
    NotificationLedgerLimitsProjection, NotificationPageProjection,
    NotificationReadStateProjection, NotificationRecordProjection,
    NotificationRetentionClassProjection, NotificationSeverityProjection,
};
pub use snapshot::{
    NotificationClearTargetProjection, NotificationSnapshot, NotificationSnapshotQuery,
    NotificationSnapshotResponse,
};
pub use version::{
    NOTIFICATION_DEFAULT_PAGE_SIZE, NOTIFICATION_PROTOCOL_VERSION, NotificationProtocolVersion,
};
