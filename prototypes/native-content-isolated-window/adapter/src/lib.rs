//! Private isolated native-window mechanism proof for Card 084.

mod adapter;
mod error;
mod process;
mod runtime;
mod wire;

pub use adapter::{IsolatedWindowAdapter, IsolatedWindowSpec};
pub use error::IsolatedWindowError;
pub use process::{
    ProcessHelperHandle, ProcessIsolatedWindowRuntime, ProcessRuntimeConfig, TeardownMode,
};
pub use runtime::{
    AdapterEvent, ChildRequest, HelperEvent, HelperEventKind, IsolatedWindowRuntime,
    RuntimeAttachRequest, RuntimeSnapshot, TeardownOutcome,
};
pub use wire::{WireCommand, WireCommandKind, WireEvent, WireEventKind};
