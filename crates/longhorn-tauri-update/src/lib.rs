//! Restart-interlocked update authorization for Longhorn applications.
//!
//! The part of the update path a consuming application could not write for
//! itself: Longhorn knows what is in flight — pending flushes, uncommitted
//! transfer sessions, running operations — and an install that relaunches
//! mid-commit is data loss.
//!
//! Installation itself is `longhorn-update-native`'s job on every host,
//! including this one — contract 018 was amended on 2026-08-09 to make
//! execution host-independent. This crate only decides whether an install
//! may proceed, and carries the reason when it may not.
//!
//! Named `tauri-*` by role rather than dependency: it takes no Tauri
//! dependency, because authorization needs none.

mod gate;
mod probes;

pub use gate::{InstallAuthorization, UpdateGate};
pub use probes::{CountingProbe, operation_probe, transfer_session_probe};
