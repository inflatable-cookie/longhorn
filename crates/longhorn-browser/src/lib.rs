//! System browser launch: the delegated capability contract 019's RFC 8252
//! flow needs, and contract 020 names.
//!
//! Host-agnostic. Neither backend supplies this — Tauri has a plugin Longhorn
//! does not take, GPUI has nothing — so Longhorn implements it once and both
//! hosts compose the same crate.
//!
//! Two independent defences, because this capability hands a
//! server-influenced string to an operating system launcher:
//!
//! - [`BrowserUrl`] is an allowlist. HTTPS with a host, ASCII, no control
//!   characters, no whitespace, no embedded credentials, bounded length.
//! - [`NativeSystemBrowser`] spawns a program directly with the URL as one
//!   argument. No shell is involved at any point.
//!
//! Either alone would probably be enough. Neither alone is the design.

mod launch;
mod url;

pub use launch::{BrowserLaunchError, DecliningSystemBrowser, NativeSystemBrowser, SystemBrowser};
pub use url::{BrowserUrl, BrowserUrlError};
