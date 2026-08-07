use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{Entitlements, Span, Timestamp};

/// How a licence's authenticity was established.
///
/// Backends differ in kind, not merely in transport, and flattening that is
/// a real defect: a signature can be re-checked at any later moment with no
/// network, where a remote assertion is only as good as its cache and cannot
/// be re-established offline. Grace policy consults this, so offline grace is
/// never granted on a basis incapable of surviving being offline.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum TrustBasis {
    /// Verified against an embedded public key, re-verifiable at any time.
    OfflineSignature {
        /// Which key verified it, so rotation can be reasoned about.
        key_id: String,
    },
    /// Asserted by a backend over an authenticated transport, then cached.
    RemoteAssertion {
        /// When the assertion was obtained.
        checked: Timestamp,
    },
}

impl TrustBasis {
    /// Returns whether this basis can be re-established with no network.
    #[must_use]
    pub const fn is_offline_verifiable(&self) -> bool {
        matches!(self, Self::OfflineSignature { .. })
    }
}

impl fmt::Display for TrustBasis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OfflineSignature { key_id } => write!(formatter, "signature by {key_id}"),
            Self::RemoteAssertion { checked } => write!(formatter, "remote assertion at {checked}"),
        }
    }
}

/// The signed content of a licence.
///
/// Two independently optional windows carry every purchase model. Nothing
/// here is named after a product, so a consumer can express one Longhorn has
/// not anticipated without a change here.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicencePayload {
    /// Which product this licence is for.
    pub product: String,
    /// Opaque reference to the customer, for support and display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_ref: Option<String>,
    /// The activation slot this licence occupies, when it occupies one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_id: Option<String>,
    /// What the licence grants.
    #[serde(default)]
    pub entitlements: Entitlements,
    /// Until when the software may be used. Absent means indefinitely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_until: Option<Timestamp>,
    /// Until when new releases may be taken. Absent means indefinitely.
    ///
    /// Independent of `use_until`, which is what makes
    /// perpetual-with-maintenance expressible: the software keeps working
    /// after this date, it just stops accepting newer releases. Contract
    /// 018's updater reads exactly this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_until: Option<Timestamp>,
    /// Until when this licence stands without revalidation.
    ///
    /// Absent means it never needs revalidating, which only makes sense for
    /// an offline-verifiable licence. Lease length is the revocation window:
    /// a signed offline licence cannot be recalled, so renewal is the only
    /// revocation there is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_until: Option<Timestamp>,
}

impl LicencePayload {
    /// Records a licence granting nothing, valid indefinitely.
    #[must_use]
    pub fn new(product: impl Into<String>) -> Self {
        Self {
            product: product.into(),
            customer_ref: None,
            activation_id: None,
            entitlements: Entitlements::new(),
            use_until: None,
            update_until: None,
            lease_until: None,
        }
    }

    /// Sets what the licence grants.
    #[must_use]
    pub fn with_entitlements(mut self, entitlements: Entitlements) -> Self {
        self.entitlements = entitlements;
        self
    }

    /// Sets the use window.
    #[must_use]
    pub const fn with_use_until(mut self, until: Timestamp) -> Self {
        self.use_until = Some(until);
        self
    }

    /// Sets the update window.
    #[must_use]
    pub const fn with_update_until(mut self, until: Timestamp) -> Self {
        self.update_until = Some(until);
        self
    }

    /// Sets the lease.
    #[must_use]
    pub const fn with_lease_until(mut self, until: Timestamp) -> Self {
        self.lease_until = Some(until);
        self
    }

    /// Sets the activation slot.
    #[must_use]
    pub fn with_activation_id(mut self, id: impl Into<String>) -> Self {
        self.activation_id = Some(id.into());
        self
    }
}

/// A licence whose authenticity has been established.
///
/// Only constructible through verification or by an adapter declaring its
/// basis, so a caller cannot fabricate one by building a struct literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedLicence {
    payload: LicencePayload,
    basis: TrustBasis,
}

impl VerifiedLicence {
    /// Records a licence an adapter established by remote assertion.
    ///
    /// Signature-verified licences come from `verify` instead. An adapter
    /// must not present a remote assertion as a signature; the two carry
    /// different offline guarantees and are treated differently.
    #[must_use]
    pub const fn from_remote_assertion(payload: LicencePayload, checked: Timestamp) -> Self {
        Self {
            payload,
            basis: TrustBasis::RemoteAssertion { checked },
        }
    }

    pub(crate) const fn from_signature(payload: LicencePayload, key_id: String) -> Self {
        Self {
            payload,
            basis: TrustBasis::OfflineSignature { key_id },
        }
    }

    /// Returns the licence content.
    #[must_use]
    pub const fn payload(&self) -> &LicencePayload {
        &self.payload
    }

    /// Returns how authenticity was established.
    #[must_use]
    pub const fn basis(&self) -> &TrustBasis {
        &self.basis
    }

    /// Returns what the licence grants.
    #[must_use]
    pub const fn entitlements(&self) -> &Entitlements {
        &self.payload.entitlements
    }

    /// Returns whether a release published now may be taken.
    ///
    /// The updater's question. A perpetual licence past its maintenance term
    /// answers `false` here while remaining perfectly usable.
    #[must_use]
    pub fn may_take_updates(&self, now: Timestamp) -> bool {
        self.payload.update_until.is_none_or(|until| now <= until)
    }
}

/// How long a lapsed lease is tolerated, by trust basis.
///
/// Two spans rather than one because the bases differ in what they can
/// promise. A remote assertion cannot be re-established without the network
/// that granted it, so extending it the same grace as a signature would be
/// claiming an offline guarantee the basis does not have.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GracePolicy {
    offline_signature: Span,
    remote_assertion: Span,
}

impl GracePolicy {
    /// Records a policy, clamping remote grace to the signature grace.
    ///
    /// The clamp is the invariant, not a convenience: a remote assertion may
    /// never be granted more tolerance than an offline signature.
    #[must_use]
    pub fn new(offline_signature: Span, remote_assertion: Span) -> Self {
        Self {
            offline_signature,
            remote_assertion: remote_assertion.min(offline_signature),
        }
    }

    /// Returns the grace applying to a basis.
    #[must_use]
    pub const fn for_basis(&self, basis: &TrustBasis) -> Span {
        match basis {
            TrustBasis::OfflineSignature { .. } => self.offline_signature,
            TrustBasis::RemoteAssertion { .. } => self.remote_assertion,
        }
    }
}

impl Default for GracePolicy {
    /// Thirty days for a signature, seven for a remote assertion.
    fn default() -> Self {
        Self::new(Span::from_days(30), Span::from_days(7))
    }
}
