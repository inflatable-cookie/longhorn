use serde::{Deserialize, Serialize};

use crate::{GracePolicy, Span, Timestamp, VerifiedLicence};

/// Guards against a clock moved backwards to defeat expiry.
///
/// Cheap and partial by design: it stops casual abuse and does not pretend
/// to stop determined abuse. Licensing is not a security boundary, and a
/// mechanism that inconveniences a user whose clock is merely wrong would
/// cost more than the abuse it prevents — hence the tolerance.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockGuard {
    /// The furthest forward this installation has ever seen the clock.
    pub highest_seen: Timestamp,
    /// How far backwards is tolerated before refusing.
    pub tolerance: Span,
}

impl ClockGuard {
    /// Records a guard with a one-day tolerance.
    ///
    /// A day absorbs timezone confusion, daylight-saving edges, and an NTP
    /// correction, none of which are abuse.
    #[must_use]
    pub const fn new(highest_seen: Timestamp) -> Self {
        Self {
            highest_seen,
            tolerance: Span::from_seconds(86_400),
        }
    }

    /// Sets how far backwards is tolerated.
    #[must_use]
    pub const fn with_tolerance(mut self, tolerance: Span) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Returns whether `now` has moved suspiciously far backwards.
    #[must_use]
    pub fn refuses(&self, now: Timestamp) -> bool {
        now.saturating_behind(self.highest_seen).as_seconds() > self.tolerance.as_seconds()
    }

    /// Returns the guard advanced to include `now`.
    #[must_use]
    pub fn observing(self, now: Timestamp) -> Self {
        Self {
            highest_seen: self.highest_seen.max(now),
            ..self
        }
    }
}

/// Whether the software may be used, and on what footing.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(export))]
pub enum Usability {
    /// Usable, with everything current.
    Active,
    /// Usable, but the lease has lapsed and renewal has not yet succeeded.
    ///
    /// Not an error and not the user's problem to solve. A backend outage
    /// must never disable a paying customer, so this state exists to be
    /// tolerated quietly rather than surfaced as a failure.
    InGrace {
        /// When grace runs out.
        until: Timestamp,
    },
    /// The use window has passed.
    UseWindowExpired {
        /// When it passed.
        at: Timestamp,
    },
    /// The lease lapsed and its grace ran out.
    LeaseLapsed {
        /// When grace ran out.
        at: Timestamp,
    },
    /// The clock moved backwards far enough to be refused.
    ClockRefused,
}

impl Usability {
    /// Returns whether the software may be used.
    ///
    /// Grace counts as usable. That is the point of it.
    #[must_use]
    pub const fn is_usable(&self) -> bool {
        matches!(self, Self::Active | Self::InGrace { .. })
    }

    /// Returns whether this state warrants telling the user something.
    ///
    /// Grace does not: a renewal that has not yet succeeded, inside its
    /// tolerance, is not something the user can act on or needs to see.
    #[must_use]
    pub const fn warrants_attention(&self) -> bool {
        !matches!(self, Self::Active | Self::InGrace { .. })
    }
}

/// Evaluates whether a licence permits use at `now`.
///
/// Ordering is deliberate. The clock guard runs first, because every
/// window comparison below it is meaningless if the clock is not trusted.
/// The use window runs before the lease, because an expired subscription is
/// a truer statement than a lapsed lease on an expired subscription.
#[must_use]
pub fn usability(
    licence: &VerifiedLicence,
    now: Timestamp,
    guard: ClockGuard,
    grace: GracePolicy,
) -> Usability {
    if guard.refuses(now) {
        return Usability::ClockRefused;
    }

    if let Some(until) = licence.payload().use_until
        && now > until
    {
        return Usability::UseWindowExpired { at: until };
    }

    let Some(lease_until) = licence.payload().lease_until else {
        // No lease means no revalidation was ever required, which only an
        // offline-verifiable licence can honestly claim.
        return Usability::Active;
    };

    if now <= lease_until {
        return Usability::Active;
    }

    let grace_ends = lease_until.saturating_add(grace.for_basis(licence.basis()));
    if now <= grace_ends {
        Usability::InGrace { until: grace_ends }
    } else {
        Usability::LeaseLapsed { at: grace_ends }
    }
}
