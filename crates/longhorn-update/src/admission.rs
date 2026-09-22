//! The exclusive admission lease: the barrier a host holds across replacement.
//!
//! Contract 018 was amended on 2026-09-22. Quiescence is a point-in-time
//! answer, and a point-in-time answer cannot keep the window between "it is
//! safe" and "the bundle is replaced" closed: work can start in between.
//! `UpdateGate` therefore acquires an **exclusive admission lease** from a
//! host-supplied [`AdmissionAuthority`] and holds it from authorization
//! through `apply`.
//!
//! # The boundary
//!
//! The application owns what "conflicting work" means, and it owns the
//! barrier that keeps new conflicting work from starting. Longhorn owns only
//! the gate: it asks for the lease, holds it, and drops it. Nothing here names
//! an application operation, and the refusal reason travels as an opaque
//! display string — Longhorn relays it and never interprets it.
//!
//! The lease releases when it is dropped, so the critical section is a Rust
//! lifetime rather than a protocol step an implementation could skip.

/// A host-supplied authority that grants exclusive admission leases.
///
/// Injected by the application, which is the only party that knows what
/// conflicts with replacing the running bundle. Implemented against the live
/// host: an authority that grants while work is in flight has moved the bug
/// into the host, and the gate cannot tell.
pub trait AdmissionAuthority {
    /// Acquires an exclusive admission lease, or refuses with the host's
    /// reason.
    ///
    /// A refusal is an ordinary deferral, not a failure and not an error to
    /// retry blindly: the gate turns it into a [`Deferral`](crate::Deferral)
    /// that carries the reason. While the returned lease is alive, the
    /// authority must refuse new conflicting work.
    fn acquire(&self) -> Result<Box<dyn AdmissionLease + '_>, AdmissionRefusal>;
}

/// An exclusive admission barrier held for the critical section.
///
/// The application grants one when nothing conflicting is in flight, and its
/// own admission rules must refuse new conflicting work for as long as the
/// value is alive. The trait is deliberately methodless: holding the value is
/// the whole capability. Dropping it releases the barrier, so a caller that
/// lets the value go early reopens the window — [`UpdateGate::authorize`]
/// returns it precisely so the controller can keep it across the installer.
///
/// [`UpdateGate::authorize`]: crate::UpdateGate::authorize
pub trait AdmissionLease {}

/// Why an admission authority would not grant an exclusive lease.
///
/// The detail is the host's display text. Longhorn carries it into the
/// deferral and never parses it, so the application can explain a conflict
/// without telling Longhorn what its operations are.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionRefusal {
    /// The host's reason, for display.
    pub detail: String,
}

impl AdmissionRefusal {
    /// Records a refusal with the host's reason.
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}
