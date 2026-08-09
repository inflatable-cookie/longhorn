//! Configuration-backed persistence for captured window placements.
//!
//! Host-neutral, despite what this comment used to say. The crate takes
//! `longhorn-config` and `longhorn-windowing` and no host adapter, so a GPUI
//! application uses it unchanged — which the composition example does. The
//! word "Tauri" here was a leftover from when there was only one host.
//!
//! Consumers retain authority over their domain schema. This crate owns the
//! repeated stage, coalesce, coordinated mutation, and synchronous flush
//! mechanics behind `WindowPlacementSink`.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

use longhorn_config::{
    BackupCatalog, ConfigDomain, ConfigStore, DomainIssue, LoadOutcome, MigrationRewriteError,
    MigrationRewriteOptions, MigrationRewriteReceipt, MutationError, MutationOptions,
    MutationReceipt, RegistrationError, StoreError,
};
use longhorn_core::WindowId;
use longhorn_windowing::{
    CapturedWindowPlacement, WindowFlushRequest, WindowPlacementFlushTicket, WindowPlacementSink,
};

type PlacementPatch<D> = dyn Fn(&mut <D as ConfigDomain>::Value, &CapturedWindowPlacement) -> Result<(), DomainIssue>
    + Send
    + Sync;

/// Registered config-domain sink with coalesced per-window staging.
pub struct ConfigWindowPlacementSink<D>
where
    D: ConfigDomain,
{
    store: ConfigStore,
    domain: D,
    mutation_options: MutationOptions,
    patch: Box<PlacementPatch<D>>,
    staged: Mutex<BTreeMap<WindowId, CapturedWindowPlacement>>,
}

impl<D> ConfigWindowPlacementSink<D>
where
    D: ConfigDomain + Send + Sync,
{
    /// Registers the domain and constructs a config-backed placement sink.
    pub fn new<P>(
        mut store: ConfigStore,
        domain: D,
        mutation_options: MutationOptions,
        patch: P,
    ) -> Result<Self, RegistrationError>
    where
        P: Fn(&mut D::Value, &CapturedWindowPlacement) -> Result<(), DomainIssue>
            + Send
            + Sync
            + 'static,
    {
        store.register(&domain)?;
        Ok(Self {
            store,
            domain,
            mutation_options,
            patch: Box::new(patch),
            staged: Mutex::new(BTreeMap::new()),
        })
    }

    /// Loads the consumer-owned domain through the registered store.
    pub fn load(&self) -> Result<LoadOutcome<D::Value>, StoreError> {
        self.store.load(&self.domain)
    }

    /// Applies a consumer-owned mutation through the same registered domain.
    pub fn mutate<F>(&self, patch: F) -> Result<MutationReceipt, MutationError>
    where
        F: FnOnce(&mut D::Value) -> Result<(), DomainIssue>,
    {
        self.store
            .mutate(&self.domain, self.mutation_options, patch)
    }

    /// Rewrites an older migrated source through Longhorn's required safety backup.
    pub fn rewrite_migrated_domain(
        &self,
        options: MigrationRewriteOptions,
    ) -> Result<MigrationRewriteReceipt, MigrationRewriteError> {
        let mut catalog = BackupCatalog::new();
        catalog
            .include(&self.domain)
            .expect("a new backup catalog accepts one registered domain");
        self.store
            .rewrite_migrated_domain(&catalog, &self.domain, options)
    }

    /// Returns the registered consumer domain.
    #[must_use]
    pub const fn domain(&self) -> &D {
        &self.domain
    }

    fn staged_for(
        &self,
        request: &WindowFlushRequest,
    ) -> Result<Vec<CapturedWindowPlacement>, String> {
        let targets: BTreeSet<_> = request
            .targets()
            .iter()
            .map(|target| target.window_id())
            .collect();
        let staged = self
            .staged
            .lock()
            .map_err(|_| "window placement staging lock is poisoned".to_string())?;
        Ok(staged
            .iter()
            .filter(|(window_id, _)| targets.contains(window_id))
            .map(|(_, placement)| placement.clone())
            .collect())
    }

    fn clear_published(&self, placements: &[CapturedWindowPlacement]) -> Result<(), String> {
        let mut staged = self
            .staged
            .lock()
            .map_err(|_| "window placement staging lock is poisoned".to_string())?;
        for placement in placements {
            if staged.get(placement.window_id()) == Some(placement) {
                staged.remove(placement.window_id());
            }
        }
        Ok(())
    }
}

impl<D> WindowPlacementSink for ConfigWindowPlacementSink<D>
where
    D: ConfigDomain + Send + Sync,
{
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String> {
        self.staged
            .lock()
            .map_err(|_| "window placement staging lock is poisoned".to_string())?
            .insert(placement.window_id().clone(), placement.clone());
        Ok(())
    }

    fn request_flush(
        &self,
        request: &WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String> {
        let placements = self.staged_for(request)?;
        if placements.is_empty() {
            return Ok(WindowPlacementFlushTicket::completed());
        }

        let result = self
            .store
            .mutate(&self.domain, self.mutation_options, |value| {
                for placement in &placements {
                    (self.patch)(value, placement)?;
                }
                Ok(())
            });
        match result {
            Ok(_) => {
                self.clear_published(&placements)?;
                Ok(WindowPlacementFlushTicket::completed())
            }
            Err(error) => Ok(WindowPlacementFlushTicket::failed(error.to_string())),
        }
    }
}
