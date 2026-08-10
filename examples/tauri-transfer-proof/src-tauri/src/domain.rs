//! Registered proof domains and the production-shaped panel adapter.

use std::{
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use longhorn_config::{
    ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath, DurabilityRequirement,
    LoadOutcome, MutationOptions, StorageClass, StorageRoots,
};
use longhorn_core::{
    DomainId, LayoutSchemaId, PanelDefinitionId, PanelInstanceId, RegionFamilyId, RegionId,
    SchemaVersion, SurfaceId, SurfaceRevision, TransferHostBindingId, WindowId,
};
use longhorn_surfaces::{
    EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutLimits, LayoutSchemaDefinition,
    PanelDefinition, PanelInstance, PanelInstancePolicy, PlacementSelector, RegionDefinition,
    RegionState, SurfaceDocument, SurfaceHostPreference, SurfaceRecord,
};
use longhorn_surfaces_config::{LayoutBackupPolicy, NoLayoutMigration, RegisteredLayoutDomain};
use longhorn_tauri_transfer::{PanelTransferAdapter, TransferCallerAuthority};
use longhorn_transfer::{
    DragSessionIdAllocationError, DragSessionIdAllocator, MonotonicClock, PanelHostBinding,
    PanelHostBindingKind, PanelHostBindings, PanelSessionAdmission, PanelSessionStartRequest,
    PanelTransferCommand, PanelTransferCommitRequest, PanelTransferCompletion,
    PanelTransferOperation, PanelTransferResponse, TargetSelector, TransferAbort,
    TransferCoordinator, TransferInstant, TransferSessionResponse, TransferSessionStarted,
    admit_panel_session, commit_panel_transfer,
};

pub(crate) const LAYOUT_DOMAIN_ID: &str = "layout.proof";
pub(crate) const SOURCE_CONTAINER_ID: &str = "container:source";
pub(crate) const TARGET_CONTAINER_ID: &str = "container:target";
pub(crate) const MAIN_REGION_ID: &str = "region:main";
pub(crate) const SOURCE_PANEL_ID: &str = "panel:tool:one";
pub(crate) const SECOND_PANEL_ID: &str = "panel:tool:two";
pub(crate) const SOURCE_BINDING_ID: &str = "binding:source";
pub(crate) const TARGET_BINDING_ID: &str = "binding:target";
pub(crate) const SOURCE_WINDOW_ID: &str = "source";
pub(crate) const TARGET_WINDOW_ID: &str = "target";

pub(crate) type LayoutDomain = RegisteredLayoutDomain<NoLayoutMigration>;

pub(crate) struct ProofDomains {
    store: Arc<ConfigStore>,
    layout: LayoutDomain,
    panel_bindings: PanelHostBindings,
    #[cfg(feature = "surface-mode")]
    surface: crate::surface::SurfaceDomain,
}

impl ProofDomains {
    pub(crate) fn new(root: &Path, binding_kind: PanelHostBindingKind) -> Result<Self, String> {
        let data = root.join("data");
        let roots = StorageRoots::new(
            create(root.join("config"))?,
            create(data.clone())?,
            create(root.join("state"))?,
            create(root.join("cache"))?,
            create(root.join("runtime"))?,
            create(root.join("log"))?,
            create(root.join("backups"))?,
        )
        .map_err(|error| error.to_string())?;
        let coordination = CoordinationAuthority::new(data).map_err(|error| error.to_string())?;
        let layout = layout_domain()?;
        #[cfg(feature = "surface-mode")]
        let surface = crate::surface::surface_domain()?;
        let mut store = ConfigStore::new(roots, coordination);
        store.register(&layout).map_err(|error| error.to_string())?;
        #[cfg(feature = "surface-mode")]
        store
            .register(&surface)
            .map_err(|error| error.to_string())?;
        let panel_bindings = panel_bindings(binding_kind, layout.descriptor().id().clone())?;
        Ok(Self {
            store: Arc::new(store),
            layout,
            panel_bindings,
            #[cfg(feature = "surface-mode")]
            surface,
        })
    }

    pub(crate) fn store(&self) -> &ConfigStore {
        &self.store
    }

    pub(crate) fn layout(&self) -> &LayoutDomain {
        &self.layout
    }

    pub(crate) fn panel_bindings(&self) -> &PanelHostBindings {
        &self.panel_bindings
    }

    pub(crate) fn layout_snapshot(&self) -> Result<SurfaceDocument, String> {
        match self
            .store
            .load(&self.layout)
            .map_err(|error| error.to_string())?
        {
            LoadOutcome::Ready(loaded) => Ok(loaded.value),
            other => Err(format!("layout proof domain is not ready: {other:?}")),
        }
    }

    #[cfg(feature = "surface-mode")]
    pub(crate) fn surface(&self) -> &crate::surface::SurfaceDomain {
        &self.surface
    }

    #[cfg(feature = "surface-mode")]
    pub(crate) fn surface_snapshot(&self) -> Result<longhorn_surfaces::SurfaceDocument, String> {
        match self
            .store
            .load(&self.surface)
            .map_err(|error| error.to_string())?
        {
            LoadOutcome::Ready(loaded) => Ok(loaded.value),
            other => Err(format!("Surface proof domain is not ready: {other:?}")),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProofClock {
    epoch: Arc<Instant>,
}

impl ProofClock {
    pub(crate) fn new() -> Self {
        Self {
            epoch: Arc::new(Instant::now()),
        }
    }
}

impl MonotonicClock for ProofClock {
    fn now(&self) -> TransferInstant {
        let millis = self.epoch.elapsed().as_millis();
        TransferInstant::new(u64::try_from(millis).unwrap_or(u64::MAX))
    }
}

pub(crate) struct ProofSessionAllocator {
    next: u64,
}

impl ProofSessionAllocator {
    pub(crate) const fn new() -> Self {
        Self { next: 1 }
    }

    #[cfg(feature = "surface-mode")]
    pub(crate) const fn surface() -> Self {
        Self { next: 1 << 63 }
    }
}

impl DragSessionIdAllocator for ProofSessionAllocator {
    fn allocate(&mut self) -> Result<[u8; 16], DragSessionIdAllocationError> {
        let value = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or(DragSessionIdAllocationError)?;
        let mut entropy = [0_u8; 16];
        entropy[..8].copy_from_slice(b"LHproof:");
        entropy[8..].copy_from_slice(&value.to_be_bytes());
        Ok(entropy)
    }
}

pub(crate) struct ProofPanelAdapter {
    domains: Arc<ProofDomains>,
    clock: ProofClock,
    allocator: ProofSessionAllocator,
}

impl ProofPanelAdapter {
    pub(crate) fn new(domains: Arc<ProofDomains>, clock: ProofClock) -> Self {
        Self {
            domains,
            clock,
            allocator: ProofSessionAllocator::new(),
        }
    }
}

impl PanelTransferAdapter for ProofPanelAdapter {
    fn start_panel(
        &mut self,
        coordinator: &mut TransferCoordinator,
        caller: TransferCallerAuthority,
        request: PanelSessionStartRequest,
    ) -> TransferSessionResponse {
        let request_id = request.request_id().clone();
        match admit_panel_session(
            self.domains.store(),
            self.domains.layout(),
            coordinator,
            &self.clock,
            &mut self.allocator,
            &self.domains.panel_bindings,
            PanelSessionAdmission::new(
                caller.window_id().clone(),
                caller.client_id().clone(),
                caller.client_epoch(),
                request.panel_instance_id().clone(),
                binding_id_for_window(caller.window_id().as_str()),
                caller.session_lifetime(),
            ),
        ) {
            Ok(receipt) => TransferSessionResponse::Started {
                session: TransferSessionStarted::from_domain(request_id, receipt),
            },
            Err(error) => TransferSessionResponse::Aborted {
                abort: TransferAbort::from_panel(request_id, &error),
            },
        }
    }

    fn commit_panel(
        &mut self,
        coordinator: &mut TransferCoordinator,
        _caller: TransferCallerAuthority,
        request: PanelTransferCommand,
        selector: TargetSelector,
        live_windows: Vec<longhorn_transfer::LiveTransferWindow>,
    ) -> PanelTransferResponse {
        let request_id = request.request_id().clone();
        match commit_panel_transfer(
            self.domains.store(),
            self.domains.layout(),
            coordinator,
            &self.clock,
            &self.domains.panel_bindings,
            mutation_options(),
            PanelTransferCommitRequest::new(
                request.session_id(),
                selector,
                live_windows,
                PanelTransferOperation::Move,
            ),
        ) {
            Ok(receipt) => PanelTransferResponse::Committed {
                completion: Box::new(PanelTransferCompletion::from_domain(request_id, &receipt)),
            },
            Err(error) => PanelTransferResponse::Aborted {
                abort: TransferAbort::from_panel(request_id, &error),
            },
        }
    }
}

pub(crate) fn binding_kind() -> PanelHostBindingKind {
    if cfg!(feature = "surface-mode") {
        PanelHostBindingKind::SurfaceContainer
    } else {
        PanelHostBindingKind::DirectWindow
    }
}

fn binding_id_for_window(window_id: &str) -> TransferHostBindingId {
    match window_id {
        SOURCE_WINDOW_ID => TransferHostBindingId::new(SOURCE_BINDING_ID),
        TARGET_WINDOW_ID => TransferHostBindingId::new(TARGET_BINDING_ID),
        _ => TransferHostBindingId::new("binding:unknown"),
    }
    .expect("proof binding constants use the opaque-id grammar")
}

fn panel_bindings(
    kind: PanelHostBindingKind,
    document_id: DomainId,
) -> Result<PanelHostBindings, String> {
    let make = |id: &str, window: &str, container: &str| match kind {
        PanelHostBindingKind::DirectWindow => PanelHostBinding::direct_window(
            TransferHostBindingId::new(id).expect("proof binding id is valid"),
            longhorn_core::WindowId::new(window).expect("proof window id is valid"),
            document_id.clone(),
            SurfaceId::new(container).expect("proof container id is valid"),
        ),
        PanelHostBindingKind::SurfaceContainer => PanelHostBinding::surface_container(
            TransferHostBindingId::new(id).expect("proof binding id is valid"),
            longhorn_core::WindowId::new(window).expect("proof window id is valid"),
            document_id.clone(),
            SurfaceId::new(container).expect("proof container id is valid"),
        ),
    };
    PanelHostBindings::new([
        make(SOURCE_BINDING_ID, SOURCE_WINDOW_ID, SOURCE_CONTAINER_ID),
        make(TARGET_BINDING_ID, TARGET_WINDOW_ID, TARGET_CONTAINER_ID),
    ])
    .map_err(|error| error.to_string())
}

fn layout_domain() -> Result<LayoutDomain, String> {
    RegisteredLayoutDomain::new(
        DomainDescriptor::new(
            DomainId::new(LAYOUT_DOMAIN_ID).expect("proof domain id is valid"),
            SchemaVersion::new(1).expect("proof schema version is valid"),
            StorageClass::MachineState,
            Some(DomainFilePath::new("proof/layout.json").expect("proof domain path is portable")),
        )
        .map_err(|error| error.to_string())?,
        layout_document(),
        layout_registry()?,
        NoLayoutMigration,
        LayoutBackupPolicy::Include,
    )
    .map_err(|error| error.to_string())
}

fn layout_registry() -> Result<LayoutDefinitionRegistry, String> {
    LayoutDefinitionRegistry::new(
        LayoutLimits::new(4, 4, 8, 4, 4, 16, 8).expect("proof limits are valid"),
        [LayoutSchemaDefinition::new(
            LayoutSchemaId::new("schema:proof").expect("proof schema id is valid"),
            [RegionDefinition::new(
                RegionId::new(MAIN_REGION_ID).expect("proof region id is valid"),
                RegionFamilyId::new("family:content").expect("proof family id is valid"),
                10,
                EmptyRegionPolicy::KeepVisible,
                false,
            )],
            [],
        )],
        [PanelDefinition::new(
            PanelDefinitionId::new("panel:tool").expect("proof panel definition id is valid"),
            [PlacementSelector::Region(
                RegionId::new(MAIN_REGION_ID).expect("proof region id is valid"),
            )],
            [PlacementSelector::Region(
                RegionId::new(MAIN_REGION_ID).expect("proof region id is valid"),
            )],
            PanelInstancePolicy::Multiple,
            true,
            true,
        )],
    )
    .map_err(|error| error.to_string())
}

fn layout_document() -> SurfaceDocument {
    let first = PanelInstanceId::new(SOURCE_PANEL_ID).expect("proof panel id is valid");
    let second = PanelInstanceId::new(SECOND_PANEL_ID).expect("proof panel id is valid");
    SurfaceDocument::new(
        SurfaceRevision::new(7),
        [
            SurfaceRecord::new(
                SurfaceId::new(SOURCE_CONTAINER_ID).expect("proof surface id is valid"),
                LayoutSchemaId::new("schema:proof").expect("proof schema id is valid"),
                None,
                [RegionState::new(
                    RegionId::new(MAIN_REGION_ID).expect("proof region id is valid"),
                    [first.clone(), second.clone()],
                    Some(first.clone()),
                    None,
                )],
                [],
                [SurfaceHostPreference::new(
                    WindowId::new(SOURCE_WINDOW_ID).expect("proof window id is valid"),
                    0,
                )],
            ),
            SurfaceRecord::new(
                SurfaceId::new(TARGET_CONTAINER_ID).expect("proof surface id is valid"),
                LayoutSchemaId::new("schema:proof").expect("proof schema id is valid"),
                None,
                [RegionState::new(
                    RegionId::new(MAIN_REGION_ID).expect("proof region id is valid"),
                    [],
                    None,
                    None,
                )],
                [],
                [SurfaceHostPreference::new(
                    WindowId::new(TARGET_WINDOW_ID).expect("proof window id is valid"),
                    0,
                )],
            ),
        ],
        [
            PanelInstance::new(
                first,
                PanelDefinitionId::new("panel:tool").expect("proof panel definition id is valid"),
            ),
            PanelInstance::new(
                second,
                PanelDefinitionId::new("panel:tool").expect("proof panel definition id is valid"),
            ),
        ],
        [],
    )
}

pub(crate) fn mutation_options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic)
}

fn create(path: impl AsRef<Path>) -> Result<std::path::PathBuf, String> {
    fs::create_dir_all(path.as_ref()).map_err(|error| error.to_string())?;
    Ok(path.as_ref().to_path_buf())
}
