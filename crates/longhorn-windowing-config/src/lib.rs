//! Configuration-backed persistence for captured window placements.
//!
//! Consumers retain authority over their domain schema. This crate owns the
//! repeated stage, coalesce, coordinated mutation, and asynchronous flush
//! mechanics behind `WindowPlacementSink`.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, mpsc},
    thread,
};

use longhorn_config::{
    BackupCatalog, ConfigDomain, ConfigStore, DomainIssue, LoadOutcome, MigrationRewriteError,
    MigrationRewriteOptions, MigrationRewriteReceipt, MutationError, MutationOptions,
    MutationReceipt, RegistrationError, StoreError,
};
use longhorn_core::WindowId;
use longhorn_windowing::{
    CapturedWindowPlacement, WindowFlushRequest, WindowPlacementFlushCompletion,
    WindowPlacementFlushTicket, WindowPlacementSink,
};

type PlacementPatch<D> = dyn Fn(&mut <D as ConfigDomain>::Value, &CapturedWindowPlacement) -> Result<(), DomainIssue>
    + Send
    + Sync;

/// Registered config-domain sink with coalesced per-window staging.
pub struct ConfigWindowPlacementSink<D>
where
    D: ConfigDomain,
{
    shared: Arc<Shared<D>>,
}

/// State a flush worker needs. Shared with the worker, so a publication can
/// outlive the caller that stopped waiting for it.
struct Shared<D>
where
    D: ConfigDomain,
{
    store: ConfigStore,
    domain: D,
    mutation_options: MutationOptions,
    patch: Box<PlacementPatch<D>>,
    /// Serializes publications: a waiting flush takes its snapshot after the
    /// previous publication has finished.
    flush: Mutex<()>,
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
            shared: Arc::new(Shared {
                store,
                domain,
                mutation_options,
                patch: Box::new(patch),
                flush: Mutex::new(()),
                staged: Mutex::new(BTreeMap::new()),
            }),
        })
    }

    /// Loads the consumer-owned domain through the registered store.
    pub fn load(&self) -> Result<LoadOutcome<D::Value>, StoreError> {
        self.shared.store.load(&self.shared.domain)
    }

    /// Applies a consumer-owned mutation through the same registered domain.
    pub fn mutate<F>(&self, patch: F) -> Result<MutationReceipt, MutationError>
    where
        F: FnOnce(&mut D::Value) -> Result<(), DomainIssue>,
    {
        self.shared
            .store
            .mutate(&self.shared.domain, self.shared.mutation_options, patch)
    }

    /// Rewrites an older migrated source through Longhorn's required safety backup.
    pub fn rewrite_migrated_domain(
        &self,
        options: MigrationRewriteOptions,
    ) -> Result<MigrationRewriteReceipt, MigrationRewriteError> {
        let mut catalog = BackupCatalog::new();
        catalog
            .include(&self.shared.domain)
            .map_err(|error| MigrationRewriteError::Preparation(error.to_string()))?;
        self.shared
            .store
            .rewrite_migrated_domain(&catalog, &self.shared.domain, options)
    }

    /// Returns the registered consumer domain.
    #[must_use]
    pub fn domain(&self) -> &D {
        &self.shared.domain
    }
}

impl<D> Shared<D>
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

    fn staged_for(
        &self,
        targets: &BTreeSet<WindowId>,
    ) -> Result<Vec<CapturedWindowPlacement>, String> {
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

    /// Publishes the staged placements for `targets`. Runs on a flush worker.
    fn publish(&self, targets: &BTreeSet<WindowId>) -> Result<(), String> {
        let _flush = self
            .flush
            .lock()
            .map_err(|_| "window placement flush lock is poisoned".to_string())?;
        let placements = self.staged_for(targets)?;
        if placements.is_empty() {
            return Ok(());
        }
        self.store
            .mutate(&self.domain, self.mutation_options, |value| {
                for placement in &placements {
                    (self.patch)(value, placement)?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        self.clear_published(&placements)
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
    D: ConfigDomain + Send + Sync + 'static,
{
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String> {
        self.shared.stage(placement)
    }

    /// Starts one publication on a worker and returns its acknowledgement.
    ///
    /// The publication does not run on the caller's thread, so the caller's
    /// wait is what bounds it. `flush_window_placements` waits on the ticket
    /// for `request.timeout()` and reports `TimedOut` when the publication
    /// outlasts that. The publication may still finish after the caller has
    /// stopped waiting. Placements stay staged until a publication succeeds,
    /// so a later flush publishes them again and nothing is lost.
    fn request_flush(
        &self,
        request: &WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String> {
        let targets: BTreeSet<WindowId> = request
            .targets()
            .iter()
            .map(|target| target.window_id().clone())
            .collect();
        // Nothing staged for these targets means nothing to publish. A
        // publication already in flight keeps its placements staged until it
        // succeeds, so an empty answer here is not a missed write.
        if self.shared.staged_for(&targets)?.is_empty() {
            return Ok(WindowPlacementFlushTicket::completed());
        }

        let (sender, receiver) = mpsc::channel();
        let shared = Arc::clone(&self.shared);
        thread::Builder::new()
            .name("longhorn-window-placement-flush".to_owned())
            .spawn(move || {
                let completion = match shared.publish(&targets) {
                    Ok(()) => WindowPlacementFlushCompletion::Succeeded,
                    Err(detail) => WindowPlacementFlushCompletion::Failed(detail),
                };
                // The caller may already have stopped waiting. A late
                // acknowledgement then has no reader, which is fine.
                let _ = sender.send(completion);
            })
            .map_err(|error| format!("could not start window placement flush: {error}"))?;
        Ok(WindowPlacementFlushTicket::new(receiver))
    }
}
