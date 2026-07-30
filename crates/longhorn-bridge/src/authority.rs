use longhorn_core::{AuthorityScopeId, DomainId};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{BridgeNegotiationError, BridgeNegotiationErrorCode};

/// Monotonic identity of the current authority tenure for a scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct AuthorityEpoch(u64);

impl AuthorityEpoch {
    /// Validates and constructs a nonzero authority epoch.
    pub fn new(value: u64) -> Result<Self, BridgeNegotiationError> {
        if value == 0 {
            Err(BridgeNegotiationError::new(
                BridgeNegotiationErrorCode::InvalidAuthorityEpoch,
                "authority epoch must be nonzero",
            ))
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the serialized epoch.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for AuthorityEpoch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u64::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Optional revision evidence reported by an authoritative domain host.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct AuthorityRevision(u64);

impl AuthorityRevision {
    /// Constructs revision evidence from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized revision.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Current availability of one domain from the negotiated host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum DomainAvailability {
    /// The domain is fully available.
    Available,
    /// The domain remains usable with a declared reduction in posture.
    Degraded,
    /// The domain is known but currently unavailable.
    Offline,
}

/// Read posture granted by the negotiated host for one domain.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ReadAuthority {
    /// No reads are available.
    None,
    /// The host exposes a non-authoritative projection.
    Projection,
    /// The host exposes the authoritative read model.
    Authoritative,
}

/// Write posture granted by the negotiated host for one domain.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum WriteAuthority {
    /// The host does not accept authoritative writes.
    None,
    /// The host is the current writer for this authority scope.
    Authoritative,
}

/// Execution ownership granted independently of domain write authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum ExecutionAuthority {
    /// The host does not own execution for this domain.
    None,
    /// The host owns execution without implying domain write authority.
    Executor,
}

/// Checked authority facts for one advertised domain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase", try_from = "RawDomainAuthorityDescriptor")]
pub struct DomainAuthorityDescriptor {
    domain_id: DomainId,
    scope_id: AuthorityScopeId,
    availability: DomainAvailability,
    read_authority: ReadAuthority,
    write_authority: WriteAuthority,
    execution_authority: ExecutionAuthority,
    authority_epoch: AuthorityEpoch,
    authoritative_revision: Option<AuthorityRevision>,
}

impl DomainAuthorityDescriptor {
    /// Validates and constructs authority facts without inferring capabilities.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        domain_id: DomainId,
        scope_id: AuthorityScopeId,
        availability: DomainAvailability,
        read_authority: ReadAuthority,
        write_authority: WriteAuthority,
        execution_authority: ExecutionAuthority,
        authority_epoch: AuthorityEpoch,
        authoritative_revision: Option<AuthorityRevision>,
    ) -> Result<Self, BridgeNegotiationError> {
        let owns_anything = read_authority != ReadAuthority::None
            || write_authority != WriteAuthority::None
            || execution_authority != ExecutionAuthority::None;
        let revision_is_authoritative = read_authority == ReadAuthority::Authoritative
            || write_authority == WriteAuthority::Authoritative;
        let valid = if availability == DomainAvailability::Offline {
            !owns_anything && authoritative_revision.is_none()
        } else {
            authoritative_revision.is_none() || revision_is_authoritative
        };

        if !valid {
            return Err(BridgeNegotiationError::new(
                BridgeNegotiationErrorCode::InvalidAuthorityDescriptor,
                format!("invalid authority posture for domain {domain_id}"),
            ));
        }

        Ok(Self {
            domain_id,
            scope_id,
            availability,
            read_authority,
            write_authority,
            execution_authority,
            authority_epoch,
            authoritative_revision,
        })
    }

    /// Returns the advertised domain.
    #[must_use]
    pub const fn domain_id(&self) -> &DomainId {
        &self.domain_id
    }

    /// Returns the independently owned authority scope.
    #[must_use]
    pub const fn scope_id(&self) -> &AuthorityScopeId {
        &self.scope_id
    }

    /// Returns current domain availability.
    #[must_use]
    pub const fn availability(&self) -> DomainAvailability {
        self.availability
    }

    /// Returns the current read posture.
    #[must_use]
    pub const fn read_authority(&self) -> ReadAuthority {
        self.read_authority
    }

    /// Returns the current write posture.
    #[must_use]
    pub const fn write_authority(&self) -> WriteAuthority {
        self.write_authority
    }

    /// Returns execution ownership independent of writes.
    #[must_use]
    pub const fn execution_authority(&self) -> ExecutionAuthority {
        self.execution_authority
    }

    /// Returns the current authority tenure.
    #[must_use]
    pub const fn authority_epoch(&self) -> AuthorityEpoch {
        self.authority_epoch
    }

    /// Returns optional authoritative revision evidence.
    #[must_use]
    pub const fn authoritative_revision(&self) -> Option<AuthorityRevision> {
        self.authoritative_revision
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawDomainAuthorityDescriptor {
    domain_id: DomainId,
    scope_id: AuthorityScopeId,
    availability: DomainAvailability,
    read_authority: ReadAuthority,
    write_authority: WriteAuthority,
    execution_authority: ExecutionAuthority,
    authority_epoch: AuthorityEpoch,
    authoritative_revision: Option<AuthorityRevision>,
}

impl TryFrom<RawDomainAuthorityDescriptor> for DomainAuthorityDescriptor {
    type Error = BridgeNegotiationError;

    fn try_from(raw: RawDomainAuthorityDescriptor) -> Result<Self, Self::Error> {
        Self::new(
            raw.domain_id,
            raw.scope_id,
            raw.availability,
            raw.read_authority,
            raw.write_authority,
            raw.execution_authority,
            raw.authority_epoch,
            raw.authoritative_revision,
        )
    }
}
