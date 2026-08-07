//! Restart-interlocked update host for Longhorn applications.
//!
//! The part of the update path a consuming application could not write for
//! itself. Longhorn knows what is in flight — pending flushes, uncommitted
//! transfer sessions, running operations — and an update that relaunches
//! mid-commit is data loss.
//!
//! Verification and installation stay inside the Tauri updater plugin.
//! Nothing here reads a signature, and that is what allows the artifact host
//! to be untrusted infrastructure.

mod gate;
mod installer;
mod probes;

pub use gate::{InstallOutcome, UpdateGate};
pub use installer::{InstallError, UpdateInstaller};
pub use probes::{CountingProbe, operation_probe, transfer_session_probe};
