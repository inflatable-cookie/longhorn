//! Authoritative same-document panel-transfer adapter.

mod admission;
mod binding;
mod commit;
mod error;
mod protocol;

pub use admission::admit_panel_session;
#[doc(hidden)]
pub use admission::{SurfaceDocumentLoadError, load_registered_surface_document};
#[doc(hidden)]
pub use binding::HostBindingIndex;
pub use binding::{PanelHostBinding, PanelHostBindingKind, PanelHostBindings};
pub use commit::commit_panel_transfer;
pub use error::{PanelTransferError, PanelTransferErrorCode};
pub use protocol::{
    PanelSessionAdmission, PanelTransferCommitReceipt, PanelTransferCommitRequest,
    PanelTransferOperation,
};
