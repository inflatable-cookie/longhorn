//! Authoritative same-document panel-transfer adapter.

mod admission;
mod binding;
mod commit;
mod error;
mod protocol;

pub use admission::admit_panel_session;
pub use binding::{PanelHostBinding, PanelHostBindingKind, PanelHostBindings};
pub use commit::commit_panel_transfer;
pub use error::{PanelTransferError, PanelTransferErrorCode};
pub use protocol::{
    PanelSessionAdmission, PanelTransferCommitReceipt, PanelTransferCommitRequest,
    PanelTransferOperation,
};
