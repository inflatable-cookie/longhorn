//! Exact metadata-only licence protocol projections.
//!
//! The domain types below this module hold verification state, signature
//! bytes, and a credential store. None of that crosses a boundary, so this
//! module restates what a client needs as owned, versioned, payload-free
//! records — the same division every other Longhorn domain draws.
//!
//! **Nothing here carries credential material outward.** A client learns that
//! a licence was verified offline or confirmed with the server; it never
//! receives a signature, a token, a key, or a key id. Credentials travel
//! inward on `LicenceActivateCommand` and stop there.
//!
//! **Entitlements stay opaque.** Longhorn answers "entitled?" and enumerates
//! no features, so an entitlement is an id and a bound. A protocol that named
//! features would be the place the milestone's line got crossed permanently.

use serde::{Deserialize, Serialize};

use crate::{Entitlements, Limit, Timestamp, TrustBasis, Usability, VerifiedLicence};

/// Exact licence protocol line.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct LicenceProtocolVersion(u32);

impl LicenceProtocolVersion {
    /// The only line this build speaks.
    pub const CURRENT: Self = Self(LICENCE_PROTOCOL_VERSION);
}

/// The protocol line this build speaks.
pub const LICENCE_PROTOCOL_VERSION: u32 = 1;

/// Whether the software may be used, as a client reads it.
///
/// Restated rather than reusing `Usability` directly, for the same reason the
/// update domain restates its availability: the projection is a boundary type
/// and must be free to keep a variant the domain later merges.
///
/// `ClockRefused` is its own state. A licence refused because the machine
/// clock moved is **not expired**, and a surface that shows "expired" for it
/// sends the operator to buy something they already own.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum LicenceUsabilityProjection {
    /// Usable, with everything current.
    Active,
    /// Usable, but the lease has lapsed and renewal has not yet succeeded.
    ///
    /// Not a failure and not the user's problem to solve. A surface that
    /// raises this as an error turns a backend outage into a support ticket
    /// from a paying customer.
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

impl LicenceUsabilityProjection {
    /// Projects one evaluated usability.
    #[must_use]
    pub const fn from_usability(usability: &Usability) -> Self {
        match usability {
            Usability::Active => Self::Active,
            Usability::InGrace { until } => Self::InGrace { until: *until },
            Usability::UseWindowExpired { at } => Self::UseWindowExpired { at: *at },
            Usability::LeaseLapsed { at } => Self::LeaseLapsed { at: *at },
            Usability::ClockRefused => Self::ClockRefused,
        }
    }
}

/// How the licence was established, without saying what established it.
///
/// The distinction is worth surfacing — "verified on this machine" and
/// "confirmed with the server ten minutes ago" are different promises — and
/// the credential behind it is not. `TrustBasis::OfflineSignature` names the
/// key that verified it so rotation can be reasoned about; that is an
/// authority-side concern and is dropped here.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum LicenceTrustBasisProjection {
    /// Verified against an embedded public key, re-verifiable with no network.
    OfflineSignature,
    /// Asserted by a backend and cached.
    RemoteAssertion {
        /// When the assertion was obtained.
        checked: Timestamp,
    },
}

impl LicenceTrustBasisProjection {
    /// Projects one trust basis, dropping the key id.
    #[must_use]
    pub const fn from_basis(basis: &TrustBasis) -> Self {
        match basis {
            TrustBasis::OfflineSignature { .. } => Self::OfflineSignature,
            TrustBasis::RemoteAssertion { checked } => Self::RemoteAssertion { checked: *checked },
        }
    }
}

/// One entitlement held, as an opaque id and its bound.
///
/// The bound is `Option<u64>` rather than `Limit`, which is an untagged enum:
/// untagged has no discriminant, so it would arrive at the boundary as a union
/// no validator can check per variant. Absent means unlimited.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LicenceEntitlementProjection {
    /// The entitlement id. Opaque: Longhorn enumerates no features.
    pub id: String,
    /// The most this entitlement permits, or absent for unlimited.
    ///
    /// Annotated as a number rather than left to bind as `bigint`. `u64` maps
    /// to `bigint` by default, and `JSON.parse` never produces one -- so the
    /// binding would describe a wire that cannot occur, and a validator
    /// written against it would reject every real payload. A seat count is
    /// small; the epochs above take the same annotation for the same reason.
    #[cfg_attr(feature = "bindings", ts(type = "number | null"))]
    pub at_most: Option<u64>,
}

impl LicenceEntitlementProjection {
    /// Projects every entitlement held, in the map's stable order.
    #[must_use]
    pub fn from_entitlements(entitlements: &Entitlements) -> Vec<Self> {
        entitlements
            .iter()
            .map(|(id, limit)| Self {
                id: id.as_str().to_owned(),
                at_most: match limit {
                    Limit::Unlimited => None,
                    Limit::AtMost(most) => Some(most),
                },
            })
            .collect()
    }
}

/// The licence held, when one is held.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HeldLicenceProjection {
    /// The product this licence covers.
    pub product: String,
    /// Whether the software may be used, and on what footing.
    pub usability: LicenceUsabilityProjection,
    /// How the licence was established.
    pub trust_basis: LicenceTrustBasisProjection,
    /// Every entitlement held.
    pub entitlements: Vec<LicenceEntitlementProjection>,
    /// When use stops being permitted, when a window applies.
    pub use_until: Option<Timestamp>,
    /// When updates stop being covered, when a window applies. Distinct from
    /// `use_until`: a perpetual licence with a lapsed update window still runs.
    pub update_until: Option<Timestamp>,
}

/// One licence state projection.
///
/// A held licence is `Option`, not a sixth usability state. "Not activated" is
/// the absence of a licence rather than a licence that cannot be used, and
/// folding it into the state union would make every consumer narrow a variant
/// that carries none of the other fields.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LicenceSnapshot {
    /// Exact metadata protocol line.
    pub protocol_version: LicenceProtocolVersion,
    /// Live authority lifetime. A plain count, as the operation, notification
    /// and update domains carry it.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
    /// The licence held, or absent when none is.
    pub licence: Option<HeldLicenceProjection>,
}

impl LicenceSnapshot {
    /// Projects a state with no licence held.
    #[must_use]
    pub const fn unlicensed(authority_epoch: u64) -> Self {
        Self {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch,
            licence: None,
        }
    }

    /// Projects a held licence evaluated at some moment.
    ///
    /// The usability is supplied rather than computed: evaluating it needs a
    /// clock, a clock guard and a grace policy, none of which this crate owns
    /// ambiently.
    #[must_use]
    pub fn held(authority_epoch: u64, licence: &VerifiedLicence, usability: &Usability) -> Self {
        let payload = licence.payload();
        Self {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch,
            licence: Some(HeldLicenceProjection {
                product: payload.product.clone(),
                usability: LicenceUsabilityProjection::from_usability(usability),
                trust_basis: LicenceTrustBasisProjection::from_basis(licence.basis()),
                entitlements: LicenceEntitlementProjection::from_entitlements(
                    licence.entitlements(),
                ),
                use_until: payload.use_until,
                update_until: payload.update_until,
            }),
        }
    }
}

/// A credential presented to activate.
///
/// One tagged union rather than three commands. They are three ways to present
/// the same thing, and three commands would make the client choose a code path
/// where the authority should.
///
/// This is the one place credential material crosses, and it crosses **inward**
/// only. Nothing in a projection carries it back.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum LicenceCredentialProjection {
    /// A redemption token typed by the customer.
    Key {
        /// The key as typed. Grouping, case and confusable symbols are the
        /// authority's problem, not the client's.
        key: String,
    },
    /// A bearer token from a completed account sign-in.
    AccountToken {
        /// The token.
        token: String,
    },
    /// A licence file the customer was sent.
    LicenceFile {
        /// The file's bytes, base64. A `Vec<u8>` would arrive as an array of
        /// numbers, which is a poor way to move a file across a boundary.
        contents_base64: String,
    },
}

/// Present a credential and ask for a licence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LicenceActivateCommand {
    /// Exact metadata protocol line.
    pub protocol_version: LicenceProtocolVersion,
    /// Authority lifetime observed by the caller.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
    /// What the customer presented.
    pub credential: LicenceCredentialProjection,
}

/// Release this machine's seat.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LicenceDeactivateCommand {
    /// Exact metadata protocol line.
    pub protocol_version: LicenceProtocolVersion,
    /// Authority lifetime observed by the caller.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
}

/// Re-check the lease now.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LicenceRefreshCommand {
    /// Exact metadata protocol line.
    pub protocol_version: LicenceProtocolVersion,
    /// Authority lifetime observed by the caller.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
}

/// Why a licence command was refused.
///
/// `NotRecognised` and `Revoked` are separate, settled 2026-08-12. They need
/// different operator actions — check your typing or your purchase, versus
/// contact support because no amount of retyping helps — and collapsing them
/// sends revoked users into a loop.
///
/// Distinguishing them lets someone learn which well-formed keys exist, which
/// only matters if keys are guessable. `LicenceKey` enforces a minimum length
/// for exactly this reason; see `key.rs`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum LicenceRejectionCode {
    /// The credential was not the right shape. A local check should normally
    /// have caught this first.
    Malformed,
    /// Well formed, and no such licence.
    NotRecognised,
    /// Recognised, and every seat is taken.
    NoSeatsFree,
    /// Recognised, and withdrawn. Retrying will not help.
    Revoked,
    /// The machine clock moved backwards far enough to be refused.
    ClockRefused,
    /// The caller's authority lifetime is behind the live one.
    StaleAuthority,
    /// The authority could not be reached. Not a licence problem, and a
    /// surface that reports it as one blames the customer for an outage.
    Unreachable,
}

/// The answer to a licence command.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "status"
)]
pub enum LicenceOutcomeProjection {
    /// The command was applied. The snapshot is the state after it.
    Committed {
        /// The state the command produced.
        snapshot: LicenceSnapshot,
    },
    /// The command was refused, and the state is unchanged.
    Rejected {
        /// What a surface should tell the operator.
        code: LicenceRejectionCode,
        /// The state as it remains.
        snapshot: LicenceSnapshot,
    },
}

/// Why a consumer's held state is stale.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum LicenceChangedKind {
    /// A licence was acquired.
    Activated,
    /// A seat was released.
    Deactivated,
    /// A lease was re-checked.
    Refreshed,
    /// Time passed and the usability state moved with it — into grace, out of
    /// it, or past a window.
    UsabilityChanged,
}

/// Non-durable invalidation hint.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LicenceChangedEvent {
    /// Exact metadata protocol line.
    pub protocol_version: LicenceProtocolVersion,
    /// Live authority lifetime.
    #[cfg_attr(feature = "bindings", ts(type = "number"))]
    pub authority_epoch: u64,
    /// Coarse invalidation category.
    pub kind: LicenceChangedKind,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClockGuard, Limit as DomainLimit};
    use crate::{EntitlementId, GracePolicy, LicencePayload, Span, usability};

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_unix_seconds(seconds)
    }

    /// A licence whose trust basis carries a key id, which is exactly the
    /// material the projection must not pass on.
    fn signed_licence() -> VerifiedLicence {
        let payload = LicencePayload::new("longhorn")
            .with_entitlements(
                crate::Entitlements::new()
                    .granting(EntitlementId::new("pro").unwrap())
                    .with(EntitlementId::new("seats").unwrap(), DomainLimit::AtMost(5)),
            )
            .with_use_until(at(2_000))
            .with_update_until(at(1_500));
        VerifiedLicence::from_signature(payload, "signing-key-2026-alpha".to_owned())
    }

    fn round_trip<T>(value: &T) -> T
    where
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let encoded = serde_json::to_string(value).expect("serialises");
        let decoded: T = serde_json::from_str(&encoded).expect("deserialises");
        assert_eq!(&decoded, value);
        decoded
    }

    #[test]
    fn every_command_round_trips() {
        round_trip(&LicenceActivateCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: 7,
            credential: LicenceCredentialProjection::Key {
                key: "ABCDE12345FGHJK6789X".to_owned(),
            },
        });
        round_trip(&LicenceActivateCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: 7,
            credential: LicenceCredentialProjection::AccountToken {
                token: "opaque".to_owned(),
            },
        });
        round_trip(&LicenceActivateCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: 7,
            credential: LicenceCredentialProjection::LicenceFile {
                contents_base64: "AAEC".to_owned(),
            },
        });
        round_trip(&LicenceDeactivateCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: 7,
        });
        round_trip(&LicenceRefreshCommand {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: 7,
        });
    }

    #[test]
    fn every_projection_round_trips() {
        let held = LicenceSnapshot::held(3, &signed_licence(), &Usability::Active);
        round_trip(&held);
        round_trip(&LicenceSnapshot::unlicensed(3));
        round_trip(&LicenceOutcomeProjection::Committed {
            snapshot: held.clone(),
        });
        round_trip(&LicenceOutcomeProjection::Rejected {
            code: LicenceRejectionCode::NoSeatsFree,
            snapshot: held,
        });
        round_trip(&LicenceChangedEvent {
            protocol_version: LicenceProtocolVersion::CURRENT,
            authority_epoch: 3,
            kind: LicenceChangedKind::UsabilityChanged,
        });
    }

    /// A licence refused because the machine clock moved is not expired. A
    /// surface that cannot tell them apart sends the operator to buy
    /// something they already own.
    #[test]
    fn clock_refused_projects_distinctly_from_every_expiry_state() {
        let state = |usability: &Usability| {
            let value = serde_json::to_value(LicenceUsabilityProjection::from_usability(usability))
                .expect("serialises");
            value["state"].as_str().expect("tagged").to_owned()
        };

        let refused = state(&Usability::ClockRefused);
        for other in [
            Usability::Active,
            Usability::InGrace { until: at(10) },
            Usability::UseWindowExpired { at: at(10) },
            Usability::LeaseLapsed { at: at(10) },
        ] {
            assert_ne!(refused, state(&other), "{other:?} must not read as refused");
        }
        assert_eq!(refused, "clockRefused");
    }

    /// The rule the whole module exists to keep. `TrustBasis` names the key
    /// that verified the licence so rotation can be reasoned about; a client
    /// has no use for it and must never receive it.
    #[test]
    fn no_credential_material_appears_in_any_projection() {
        let licence = signed_licence();
        let held = LicenceSnapshot::held(1, &licence, &Usability::Active);

        let encoded = [
            serde_json::to_string(&held).expect("serialises"),
            serde_json::to_string(&LicenceOutcomeProjection::Committed {
                snapshot: held.clone(),
            })
            .expect("serialises"),
            serde_json::to_string(&LicenceOutcomeProjection::Rejected {
                code: LicenceRejectionCode::Revoked,
                snapshot: held,
            })
            .expect("serialises"),
        ]
        .join("");

        assert!(
            !encoded.contains("signing-key-2026-alpha"),
            "the key id reached a projection: {encoded}"
        );
        for forbidden in ["keyId", "signature", "token", "credential"] {
            assert!(
                !encoded.contains(forbidden),
                "`{forbidden}` reached a projection: {encoded}"
            );
        }
    }

    /// Longhorn enumerates no features, so an entitlement is an id and a
    /// bound. `Limit` is untagged in the domain; the projection flattens it
    /// rather than sending a union no validator can check per variant.
    #[test]
    fn entitlements_project_as_opaque_ids_with_their_bounds() {
        let held = LicenceSnapshot::held(1, &signed_licence(), &Usability::Active)
            .licence
            .expect("held");

        assert_eq!(
            held.entitlements,
            vec![
                LicenceEntitlementProjection {
                    id: "pro".to_owned(),
                    at_most: None,
                },
                LicenceEntitlementProjection {
                    id: "seats".to_owned(),
                    at_most: Some(5),
                },
            ]
        );
    }

    /// The two windows answer different questions. A perpetual licence past
    /// its maintenance term still runs, and one field cannot say that.
    #[test]
    fn the_use_and_update_windows_are_separate_fields() {
        let held = LicenceSnapshot::held(1, &signed_licence(), &Usability::Active)
            .licence
            .expect("held");

        assert_eq!(held.use_until, Some(at(2_000)));
        assert_eq!(held.update_until, Some(at(1_500)));
    }

    /// Not activated is the absence of a licence, not a usability state.
    #[test]
    fn an_unlicensed_snapshot_holds_no_licence() {
        assert!(LicenceSnapshot::unlicensed(9).licence.is_none());
        assert_eq!(LicenceSnapshot::unlicensed(9).authority_epoch, 9);
    }

    /// The projection is fed by the domain's own evaluation rather than
    /// re-deriving it, so the two cannot disagree about grace.
    #[test]
    fn grace_projects_as_usable_with_its_deadline() {
        let payload = LicencePayload::new("longhorn").with_lease_until(at(100));
        let licence = VerifiedLicence::from_remote_assertion(payload, at(90));
        let grace = GracePolicy::new(Span::from_seconds(60), Span::from_seconds(30));
        let evaluated = usability(&licence, at(110), ClockGuard::new(at(110)), grace);

        assert_eq!(
            LicenceUsabilityProjection::from_usability(&evaluated),
            LicenceUsabilityProjection::InGrace { until: at(130) }
        );
    }
}
