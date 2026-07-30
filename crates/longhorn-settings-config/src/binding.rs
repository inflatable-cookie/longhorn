use std::{collections::BTreeSet, error::Error, fmt};

use longhorn_config::{ConfigDomain, StorageClass};
use longhorn_core::{DomainId, SettingsApplyUnitId, SettingsPageId};
use longhorn_settings::{
    SettingsApplyUnitDefinition, SettingsLimits, SettingsRegistry, SettingsRegistryGeneration,
};

use crate::{SettingsConfigAdapter, authority::AuthorityTracker};

/// One sealed apply unit bound to exactly one configuration domain.
pub struct ConfigSettingsApplyUnit<D, A>
where
    D: ConfigDomain,
    A: SettingsConfigAdapter<D::Value>,
{
    pub(crate) generation: SettingsRegistryGeneration,
    pub(crate) limits: SettingsLimits,
    pub(crate) definition: SettingsApplyUnitDefinition,
    pub(crate) authorized_pages: BTreeSet<SettingsPageId>,
    pub(crate) domain: D,
    pub(crate) adapter: A,
    pub(crate) authority: AuthorityTracker,
}

impl<D, A> ConfigSettingsApplyUnit<D, A>
where
    D: ConfigDomain,
    A: SettingsConfigAdapter<D::Value>,
{
    /// Binds one admitted settings unit to one ordinary writable domain.
    pub fn new(
        registry: &SettingsRegistry,
        apply_unit_id: &SettingsApplyUnitId,
        domain: D,
        adapter: A,
    ) -> Result<Self, ConfigSettingsBindingError> {
        let definition = registry.apply_unit(apply_unit_id).cloned().ok_or_else(|| {
            ConfigSettingsBindingError::UnknownApplyUnit {
                apply_unit_id: apply_unit_id.clone(),
            }
        })?;
        let storage_class = domain.descriptor().storage_class();
        if !matches!(
            storage_class,
            StorageClass::UserConfig | StorageClass::MachineState | StorageClass::WorkspaceLocal
        ) {
            return Err(ConfigSettingsBindingError::UnsupportedStorageClass {
                domain_id: domain.descriptor().id().clone(),
                storage_class,
            });
        }

        let authorized_pages = registry
            .pages()
            .filter(|page| page.writable_apply_unit_ids.contains(apply_unit_id))
            .map(|page| page.id.clone())
            .collect::<BTreeSet<_>>();
        if authorized_pages.is_empty() {
            return Err(ConfigSettingsBindingError::NoAuthorizedPage {
                apply_unit_id: apply_unit_id.clone(),
            });
        }

        let authority = AuthorityTracker::new(registry.generation(), definition.scope_id.clone());
        Ok(Self {
            generation: registry.generation(),
            limits: registry.limits(),
            definition,
            authorized_pages,
            domain,
            adapter,
            authority,
        })
    }

    /// Returns the exact bound domain identity.
    #[must_use]
    pub fn domain_id(&self) -> &DomainId {
        self.domain.descriptor().id()
    }

    /// Returns the exact single domain bound to this unit.
    #[must_use]
    pub const fn domain(&self) -> &D {
        &self.domain
    }

    /// Returns the sealed apply-unit declaration.
    #[must_use]
    pub const fn definition(&self) -> &SettingsApplyUnitDefinition {
        &self.definition
    }
}

/// Invalid sealed-registry to configuration-domain binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigSettingsBindingError {
    /// The sealed registry does not contain the named apply unit.
    UnknownApplyUnit {
        /// Missing unit.
        apply_unit_id: SettingsApplyUnitId,
    },
    /// No admitted page is authorized to invoke the unit.
    NoAuthorizedPage {
        /// Unreachable unit.
        apply_unit_id: SettingsApplyUnitId,
    },
    /// The built-in adapter cannot claim this storage authority.
    UnsupportedStorageClass {
        /// Bound domain.
        domain_id: DomainId,
        /// Refused storage authority.
        storage_class: StorageClass,
    },
}

impl fmt::Display for ConfigSettingsBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownApplyUnit { apply_unit_id } => {
                write!(formatter, "unknown settings apply unit {apply_unit_id}")
            }
            Self::NoAuthorizedPage { apply_unit_id } => {
                write!(
                    formatter,
                    "settings apply unit {apply_unit_id} has no authorized page"
                )
            }
            Self::UnsupportedStorageClass {
                domain_id,
                storage_class,
            } => write!(
                formatter,
                "settings domain {domain_id} uses unsupported {storage_class:?} authority"
            ),
        }
    }
}

impl Error for ConfigSettingsBindingError {}
