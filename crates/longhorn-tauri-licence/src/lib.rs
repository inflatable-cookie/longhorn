//! Narrow Tauri host assembly for the injected licence authority.
//!
//! Nothing here decides anything. The consumer composes the authority —
//! `longhorn-licence` for the rules, `longhorn-credential-keyring` for
//! storage, `longhorn-browser` for the RFC 8252 round trip — and this crate
//! carries the protocol's commands across the Tauri boundary and back, the
//! same seam `longhorn-tauri-update` is for its domain.
//!
//! Credential material crosses this seam **inward only**, on the activate
//! command. Every outbound type is a projection Card 193 built to carry no
//! credential, and the client's validator refuses one that does.

mod authority;
mod commands;
mod error;
mod handler;

pub use authority::LicenceHostAuthority;
pub use commands::{
    LICENCE_CHANGED_EVENT, LicenceHostService, TauriLicenceState, licence_changed_event,
    longhorn_licence_activate, longhorn_licence_deactivate, longhorn_licence_refresh,
    longhorn_licence_release_seat, longhorn_licence_rename_seat, longhorn_licence_snapshot,
    publish_licence_changed,
};
pub use error::{LicenceHostError, LicenceHostErrorCode};
pub use handler::LicenceHandlerAssembly;
