//! Private child-webview mechanism proof for Card 083.

mod adapter;
mod error;
mod policy;
mod runtime;
mod tauri_runtime;

pub use adapter::{ChildWebviewAdapter, InvalidatedAttachment};
pub use error::ChildWebviewError;
pub use policy::{
    ChildWebviewLabel, ChildWebviewSpec, DownloadPolicy, PopupPolicy, RemoteCapabilityPolicy,
};
pub use runtime::{
    AdapterEvent, ChildWebviewRuntime, RuntimeAttachRequest, RuntimeEvent, RuntimeEventKind,
};
pub use tauri_runtime::TauriChildWebviewRuntime;
