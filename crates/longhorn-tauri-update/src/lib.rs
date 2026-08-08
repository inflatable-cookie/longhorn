//! Restart-interlocked update authorization for Longhorn applications.
//!
//! The part of the update path a consuming application could not write for
//! itself: Longhorn knows what is in flight — pending flushes, uncommitted
//! transfer sessions, running operations — and an install that relaunches
//! mid-commit is data loss.
//!
//! Installation itself is the Tauri updater plugin's job: check, download,
//! signature verification, and bundle replacement all stay inside the
//! plugin. This crate only decides whether an install may proceed, and
//! carries the reason when it may not.

mod gate;
mod probes;

pub use gate::{InstallAuthorization, UpdateGate};
pub use probes::{CountingProbe, operation_probe, transfer_session_probe};
