//! Tauri child-view execution for production native-content coordination.
//!
//! Browser and capability policy are injected. Raw Tauri webview handles stay
//! behind the selected runtime and never enter the renderer protocol.

mod adapter;
mod error;
mod policy;
mod runtime;
mod tauri_runtime;

pub use adapter::{
    ChildViewAdapter, ChildViewHostDestroyOutcome, ChildViewHostDestroyReceipt,
    ChildViewNavigationOutcome, ChildViewNavigationReceipt, ChildViewTeardownOutcome,
    ChildViewTeardownReceipt,
};
pub use error::ChildViewError;
pub use policy::{
    CHILD_VIEW_CAPABILITIES, ChildViewLabel, ChildViewPolicyEvent, ChildViewPolicyHooks,
    ChildViewSpec,
};
pub use runtime::{
    ChildViewAdapterEvent, ChildViewRuntime, ChildViewRuntimeEvent, ChildViewRuntimeEventKind,
    RuntimeAttachRequest,
};
pub use tauri_runtime::{TauriChildViewHandle, TauriChildViewRuntime};
