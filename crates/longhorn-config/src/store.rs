mod checked;
pub(crate) mod document;
pub(crate) mod load;
pub(super) mod mutation;
pub(crate) mod publication;
mod types;

pub use checked::{CheckedMutationContext, CheckedMutationError, CheckedMutationOutcome};
pub use types::{
    CoordinatedLoadError, Durability, DurabilityRequirement, LoadDiagnostic, LoadDiagnosticCode,
    LoadOutcome, LoadedConfig, LoadedOrigin, MutationError, MutationOptions, MutationReceipt,
    MutationRefusal, PublicationFailure, PublicationStage, RecoveryKind, RecoveryState,
    SourceDocument, StoreError, UnavailableState,
};

use crate::{
    BackupApplication, BackupArchiveInspection, BackupCaptureError, BackupCaptureOptions,
    BackupCatalog, BackupMetadata, BackupProducer, BackupScope, BackupSnapshot, ConfigDomain,
    CoordinationAuthority, DomainIssue, DomainLocation, MigrationRewriteError,
    MigrationRewriteOptions, MigrationRewriteReceipt, RegistrationError, RestoreAdapterGroupError,
    RestoreAdapterGroupExecutionOptions, RestoreAdapterGroupExecutionReceipt,
    RestoreAdapterGroupPlan, RestoreAdapterGroupPlanError, RestoreAdapterGroupRecoveryError,
    RestoreAdapterGroupRecoveryReceipt, RestoreChoices, RestoreExecutionError,
    RestoreExecutionOptions, RestoreExecutionReceipt, RestoreInspection, RestoreOperationState,
    RestorePlan, RestorePlanError, RestorePrepareError, RestorePrepareOptions,
    RestoreRecoveryError, RestoreRecoveryOptions, RestoreRecoveryReceipt, RestoreStaging,
    Sha256Digest, StorageRoots, coordination::Coordinator, registry::DomainRegistry,
};

use self::load::{load_file, validated_default};

/// Read-only view held under the store coordinator for one stable file generation.
pub struct CoordinatedLoadSet<'store> {
    store: &'store ConfigStore,
}

impl CoordinatedLoadSet<'_> {
    /// Loads one registered domain without releasing the load-set coordinator.
    pub fn load<D: ConfigDomain>(&self, domain: &D) -> Result<LoadOutcome<D::Value>, StoreError> {
        self.store.ensure_registered(domain)?;
        Ok(self.store.load_registered(domain))
    }
}

/// Registered configuration domain store.
#[derive(Debug)]
pub struct ConfigStore {
    pub(super) roots: StorageRoots,
    registry: DomainRegistry,
    pub(super) coordinator: Coordinator,
}

impl ConfigStore {
    /// Constructs an empty store over injected roots and coordination authority.
    #[must_use]
    pub fn new(roots: StorageRoots, coordination: CoordinationAuthority) -> Self {
        Self {
            roots,
            registry: DomainRegistry::default(),
            coordinator: Coordinator::new(coordination),
        }
    }

    /// Registers a typed domain descriptor.
    pub fn register<D: ConfigDomain>(&mut self, domain: &D) -> Result<(), RegistrationError> {
        self.registry.register(&self.roots, domain.descriptor())
    }

    /// Returns the registered domain's typed location.
    pub fn location<D: ConfigDomain>(&self, domain: &D) -> Result<DomainLocation, StoreError> {
        self.ensure_registered(domain)?;
        Ok(self.roots.resolve(domain.descriptor()))
    }

    /// Loads, validates, and when required migrates a registered domain.
    pub fn load<D: ConfigDomain>(&self, domain: &D) -> Result<LoadOutcome<D::Value>, StoreError> {
        self.ensure_registered(domain)?;
        match self.restore_operation_state() {
            RestoreOperationState::Active => {
                // A journal already in a terminal phase is a
                // crash-after-completion artifact; heal it the way
                // coordinated load-sets do instead of wedging every bare
                // load until someone calls recovery explicitly.
                if !self.try_self_heal_terminal_restore() {
                    return Ok(LoadOutcome::Unavailable(UnavailableState::RestoreActive));
                }
            }
            RestoreOperationState::RecoveryRequired => {
                return Ok(LoadOutcome::Unavailable(
                    UnavailableState::RestoreRecoveryRequired,
                ));
            }
            RestoreOperationState::Inactive => {}
        }
        Ok(self.load_registered(domain))
    }

    /// Non-blocking best-effort cleanup of a terminal restore journal.
    /// Returns whether loads may proceed. Contended coordination, grouped
    /// journals, and genuinely active restores all leave the journal alone.
    fn try_self_heal_terminal_restore(&self) -> bool {
        if !crate::backup::restore::ordinary_terminal_phase_pending(self) {
            return false;
        }
        let Ok(guard) = self.coordinator.acquire(std::time::Duration::ZERO) else {
            return false;
        };
        if crate::backup::restore::recover_guarded(self, &guard).is_err() {
            return false;
        }
        matches!(
            self.restore_operation_state(),
            RestoreOperationState::Inactive
        )
    }

    fn load_registered<D: ConfigDomain>(&self, domain: &D) -> LoadOutcome<D::Value> {
        let location = self.roots.resolve(domain.descriptor());

        match location {
            DomainLocation::DefaultsOnly => validated_default(domain, None, false),
            DomainLocation::SecureStoreRequired | DomainLocation::RootRequired { .. } => {
                LoadOutcome::Unavailable(UnavailableState::Authority { location })
            }
            DomainLocation::File(file) => load_file(domain, &file),
        }
    }

    /// Captures a bounded immutable snapshot of selected published domains.
    pub fn capture_backup(
        &self,
        catalog: &BackupCatalog<'_>,
        scope: &BackupScope,
        metadata: BackupMetadata,
        options: BackupCaptureOptions,
    ) -> Result<BackupSnapshot, BackupCaptureError> {
        crate::backup::capture::capture(self, catalog, scope, metadata, options)
    }

    /// Inspects a verified archive without reading or mutating current files.
    #[must_use]
    pub fn inspect_restore(
        &self,
        catalog: &BackupCatalog<'_>,
        archive: &BackupArchiveInspection,
        application: &BackupApplication,
        producer: &BackupProducer,
    ) -> RestoreInspection {
        crate::backup::restore::inspect(self, catalog, archive, application, producer)
    }

    /// Executes one explicitly confirmed adapter-owned restore operation.
    ///
    /// Custom domains never enter the ordinary file transaction implicitly.
    /// Requiring failure atomicity refuses separately receipted adapters before
    /// mutation.
    pub fn execute_adapter_restore(
        &self,
        catalog: &BackupCatalog<'_>,
        archive: &BackupArchiveInspection,
        inspection: &RestoreInspection,
        domain: &longhorn_core::DomainId,
        confirmation: &Sha256Digest,
        requirement: crate::RestoreAdapterRequirement,
    ) -> Result<crate::RestoreAdapterReceipt, crate::RestoreAdapterError> {
        crate::backup::restore::execute_adapter(
            self,
            catalog,
            archive,
            inspection,
            domain,
            confirmation,
            requirement,
        )
    }

    /// Binds one inspected archive and exact selected custom-domain set into a
    /// single grouped confirmation.
    pub fn plan_grouped_adapter_restore(
        &self,
        inspection: &RestoreInspection,
        domains: impl IntoIterator<Item = longhorn_core::DomainId>,
    ) -> Result<RestoreAdapterGroupPlan, RestoreAdapterGroupPlanError> {
        crate::backup::restore::plan_grouped_adapters(inspection, domains)
    }

    /// Stages, journals, applies, and verifies one complete grouped custom
    /// restore. The consumer must quiesce external live authorities first.
    pub fn execute_grouped_adapter_restore(
        &self,
        catalog: &BackupCatalog<'_>,
        archive: &BackupArchiveInspection,
        inspection: &RestoreInspection,
        plan: &RestoreAdapterGroupPlan,
        confirmation: &Sha256Digest,
        options: RestoreAdapterGroupExecutionOptions,
    ) -> Result<RestoreAdapterGroupExecutionReceipt, RestoreAdapterGroupError> {
        crate::backup::restore::execute_grouped_adapters(
            self,
            catalog,
            archive,
            inspection,
            plan,
            confirmation,
            options,
        )
    }

    /// Rolls a journalled grouped adapter restore back through the exact boot
    /// catalogue before consumer authorities open.
    pub fn recover_grouped_adapter_restore(
        &self,
        catalog: &BackupCatalog<'_>,
        lock_timeout: std::time::Duration,
    ) -> Result<RestoreAdapterGroupRecoveryReceipt, RestoreAdapterGroupRecoveryError> {
        crate::backup::restore::recover_grouped_adapters(self, catalog, lock_timeout)
    }

    /// Binds explicit conflict choices and exact current evidence into a plan.
    pub fn plan_restore(
        &self,
        inspection: &RestoreInspection,
        choices: &RestoreChoices,
    ) -> Result<RestorePlan, RestorePlanError> {
        crate::backup::restore::plan(self, inspection, choices)
    }

    /// Rechecks confirmed evidence under coordination and stages every target privately.
    pub fn prepare_restore(
        &self,
        catalog: &BackupCatalog<'_>,
        archive: &BackupArchiveInspection,
        inspection: &RestoreInspection,
        plan: &RestorePlan,
        options: RestorePrepareOptions,
    ) -> Result<RestoreStaging, RestorePrepareError> {
        crate::backup::restore::prepare(self, catalog, archive, inspection, plan, options)
    }

    /// Executes a fully staged restore through a durable journal and exact rollback set.
    pub fn execute_restore(
        &self,
        catalog: &BackupCatalog<'_>,
        staging: RestoreStaging,
        options: RestoreExecutionOptions,
    ) -> Result<RestoreExecutionReceipt, RestoreExecutionError> {
        crate::backup::restore::execute(self, catalog, staging, options)
    }

    /// Rolls back and verifies any interrupted destructive operation.
    pub fn recover_restore(
        &self,
        options: RestoreRecoveryOptions,
    ) -> Result<RestoreRecoveryReceipt, RestoreRecoveryError> {
        crate::backup::restore::recover(self, options.lock_timeout)
    }

    /// Returns the lock-free durable restore-journal state.
    #[must_use]
    pub fn restore_operation_state(&self) -> RestoreOperationState {
        crate::backup::restore::operation_state(self)
    }

    /// Returns the safety archive digest pinned by an unresolved journal.
    pub fn restore_safety_pin(&self) -> Result<Option<Sha256Digest>, RestoreRecoveryError> {
        crate::backup::restore::safety_pin(self)
    }

    /// Rewrites one in-memory-migrated source through the safety-backup journal boundary.
    pub fn rewrite_migrated_domain<D: ConfigDomain>(
        &self,
        catalog: &BackupCatalog<'_>,
        domain: &D,
        options: MigrationRewriteOptions,
    ) -> Result<MigrationRewriteReceipt, MigrationRewriteError> {
        crate::backup::restore::rewrite_migration(self, catalog, domain, options)
    }

    /// Runs multiple loads while holding one generation-stable coordinator guard.
    pub fn with_coordinated_load_set<R>(
        &self,
        lock_timeout: std::time::Duration,
        read: impl FnOnce(&CoordinatedLoadSet<'_>) -> R,
    ) -> Result<R, CoordinatedLoadError> {
        let guard = self
            .coordinator
            .acquire(lock_timeout)
            .map_err(CoordinatedLoadError::Coordination)?;
        crate::backup::restore::recover_guarded(self, &guard)
            .map_err(CoordinatedLoadError::RestoreRecovery)?;
        Ok(read(&CoordinatedLoadSet { store: self }))
    }

    /// Applies and atomically publishes a patch against the latest valid value.
    pub fn mutate<D, F>(
        &self,
        domain: &D,
        options: MutationOptions,
        patch: F,
    ) -> Result<MutationReceipt, MutationError>
    where
        D: ConfigDomain,
        F: FnOnce(&mut D::Value) -> Result<(), DomainIssue>,
    {
        mutation::mutate(self, domain, options, patch)
    }

    /// Checks and patches fresh state under one coordinator acquisition.
    ///
    /// A check failure publishes nothing. An accepted patch is validated and
    /// atomically published unless its encoded bytes are unchanged.
    pub fn mutate_checked<D, R, E, F>(
        &self,
        domain: &D,
        options: MutationOptions,
        check_and_patch: F,
    ) -> Result<CheckedMutationOutcome<R, D::Value>, CheckedMutationError<E>>
    where
        D: ConfigDomain,
        F: FnOnce(&mut CheckedMutationContext<'_, D::Value>) -> Result<R, E>,
    {
        mutation::mutate_checked(self, domain, options, check_and_patch)
    }

    pub(super) fn ensure_registered<D: ConfigDomain>(&self, domain: &D) -> Result<(), StoreError> {
        let descriptor = domain.descriptor();
        let Some(registered) = self.registry.descriptor(descriptor.id()) else {
            return Err(StoreError::NotRegistered {
                id: descriptor.id().clone(),
            });
        };
        if registered != descriptor {
            return Err(StoreError::DescriptorChanged {
                id: descriptor.id().clone(),
            });
        }
        Ok(())
    }

    pub(crate) fn registered_descriptor(
        &self,
        id: &longhorn_core::DomainId,
    ) -> Option<&crate::DomainDescriptor> {
        self.registry.descriptor(id)
    }

    pub(crate) fn registered_descriptors(&self) -> impl Iterator<Item = &crate::DomainDescriptor> {
        self.registry.descriptors()
    }
}
