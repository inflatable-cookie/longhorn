//! Optional Surface hierarchy, transfer adapter, and real window provisioner.

use std::sync::Arc;
use std::time::Duration;

use longhorn_config::{
    DomainDescriptor, DomainFilePath, DurabilityRequirement, MutationOptions, StorageClass,
};
use longhorn_core::{
    DisplayId, DomainId, LayoutSchemaId, PhysicalPx, RoundingMode, SchemaVersion, ScreenPoint,
    ScreenRect, ScreenSize, SurfaceId, SurfaceRevision, TransferHostBindingId, WindowId,
    WindowPlacement,
};
use longhorn_surface_transfer::{
    EmptyDisplayProvisionPolicy, EmptyDisplayProvisionTarget, ProvisionedSurfaceWindow,
    SurfaceHostBinding, SurfaceHostBindings, SurfaceSessionAdmission, SurfaceSessionResponse,
    SurfaceSessionStartRequest, SurfaceTransferAbort, SurfaceTransferCommand,
    SurfaceTransferCommitRequest, SurfaceTransferCompletion, SurfaceTransferPolicy,
    SurfaceTransferResponse, SurfaceWindowCleanupReceipt, SurfaceWindowCommitReceipt,
    SurfaceWindowProvisionFailure, SurfaceWindowProvisionReceipt, SurfaceWindowProvisionRequest,
    SurfaceWindowProvisionStage, SurfaceWindowProvisioner, admit_surface_session,
    commit_surface_transfer,
};
use longhorn_surfaces::{
    EmptyWindowPolicy, ParticipatingWindow, SurfaceDocument, SurfaceHostPreference, SurfaceLimits,
    SurfaceRecord,
};
use longhorn_surfaces_config::{NoSurfaceMigration, RegisteredSurfaceDomain, SurfaceBackupPolicy};
use longhorn_tauri_transfer::{SurfaceTransferAdapter, TransferCallerAuthority};
use longhorn_tauri_windowing::{
    DefaultDisplayMetadata, TauriDesktopReadback, TauriWindowHost, TauriWindowMutationBackend,
    UniformScaleMapper, observe_tauri_desktop, scale_factor_from_tauri,
};
use longhorn_transfer::{TargetSelector, TransferCoordinator, TransferSessionStarted};
use longhorn_windowing::{ApplyGeneration, DesiredWindow, WindowDiffInput};
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder, Wry,
};

use crate::domain::{
    ProofClock, ProofDomains, ProofSessionAllocator, SOURCE_CONTAINER_ID, SOURCE_WINDOW_ID,
    TARGET_CONTAINER_ID, TARGET_WINDOW_ID,
};

pub(crate) const SURFACE_DOMAIN_ID: &str = "surfaces.proof";
pub(crate) const SOURCE_SURFACE_ID: &str = "surface:source";
pub(crate) const SECOND_SURFACE_ID: &str = "surface:second";
pub(crate) const PROVISIONED_WINDOW_ID: &str = "provisioned";
pub(crate) const PROVISIONED_BINDING_ID: &str = "binding:provisioned";

pub(crate) type SurfaceDomain = RegisteredSurfaceDomain<NoSurfaceMigration>;

#[derive(Clone, Debug)]
pub(crate) struct ScreenPolicy {
    display_id: DisplayId,
    display_bounds: ScreenRect,
    drop_point: ScreenPoint,
    placement: WindowPlacement,
}

impl ScreenPolicy {
    pub(crate) fn from_app(app: &AppHandle<Wry>) -> Result<Self, String> {
        let monitor = app
            .primary_monitor()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "packaged proof requires a primary monitor".to_string())?;
        let scale =
            scale_factor_from_tauri(monitor.scale_factor()).map_err(|error| error.to_string())?;
        let origin = monitor.position();
        let size = monitor.size();
        let x = scale
            .physical_to_screen(PhysicalPx::new(origin.x), RoundingMode::Nearest)
            .map_err(|error| error.to_string())?
            .get();
        let y = scale
            .physical_to_screen(PhysicalPx::new(origin.y), RoundingMode::Nearest)
            .map_err(|error| error.to_string())?
            .get();
        let width = physical_extent(size.width, scale)?;
        let height = physical_extent(size.height, scale)?;
        let bounds = ScreenRect::new(ScreenPoint::new(x, y), ScreenSize::new(width, height));
        let drop_point = ScreenPoint::new(
            x.saturating_add(i32::try_from(width).unwrap_or(i32::MAX))
                .saturating_sub(24),
            y.saturating_add(i32::try_from(height).unwrap_or(i32::MAX))
                .saturating_sub(24),
        );
        let window_width = width.saturating_sub(80).clamp(240, 480);
        let window_height = height.saturating_sub(80).clamp(200, 360);
        let placement = WindowPlacement::new(
            ScreenPoint::new(
                x.saturating_add(
                    i32::try_from(width.saturating_sub(window_width) / 2).unwrap_or(0),
                ),
                y.saturating_add(
                    i32::try_from(height.saturating_sub(window_height) / 2).unwrap_or(0),
                ),
            ),
            ScreenSize::new(window_width, window_height),
        );
        Ok(Self {
            display_id: DisplayId::new("display:primary")
                .expect("proof display id uses the opaque-id grammar"),
            display_bounds: bounds,
            drop_point,
            placement,
        })
    }

    pub(crate) const fn drop_point(&self) -> ScreenPoint {
        self.drop_point
    }

    pub(crate) const fn display_bounds(&self) -> ScreenRect {
        self.display_bounds
    }

    pub(crate) const fn placement(&self) -> WindowPlacement {
        self.placement
    }

    fn transfer_policy(&self) -> SurfaceTransferPolicy {
        SurfaceTransferPolicy::new(
            [
                window_id(TARGET_WINDOW_ID),
                window_id(PROVISIONED_WINDOW_ID),
            ],
            EmptyWindowPolicy::Allow,
            EmptyDisplayProvisionPolicy::Enabled(vec![EmptyDisplayProvisionTarget::new(
                self.display_id.clone(),
                self.display_bounds,
                window_id(PROVISIONED_WINDOW_ID),
                self.placement,
                None,
            )]),
        )
        .expect("proof Surface policy is valid")
    }
}

pub(crate) struct ProofSurfaceAdapter {
    domains: Arc<ProofDomains>,
    clock: ProofClock,
    allocator: ProofSessionAllocator,
    bindings: SurfaceHostBindings,
    policy: SurfaceTransferPolicy,
    provisioner: ProofProvisioner,
}

impl ProofSurfaceAdapter {
    pub(crate) fn new(
        app: AppHandle<Wry>,
        window_host: Arc<TauriWindowHost<Wry>>,
        domains: Arc<ProofDomains>,
        clock: ProofClock,
        screen: &ScreenPolicy,
    ) -> Self {
        Self {
            domains,
            clock,
            allocator: ProofSessionAllocator::surface(),
            bindings: surface_bindings(),
            policy: screen.transfer_policy(),
            provisioner: ProofProvisioner::new(app, window_host),
        }
    }
}

impl SurfaceTransferAdapter for ProofSurfaceAdapter {
    fn start_surface(
        &mut self,
        coordinator: &mut TransferCoordinator,
        caller: TransferCallerAuthority,
        request: SurfaceSessionStartRequest,
    ) -> SurfaceSessionResponse {
        let request_id = request.request_id().clone();
        match admit_surface_session(
            self.domains.store(),
            self.domains.surface(),
            coordinator,
            &self.clock,
            &mut self.allocator,
            &self.bindings,
            SurfaceSessionAdmission::new(
                caller.window_id().clone(),
                caller.client_id().clone(),
                caller.client_epoch(),
                request.surface_id().clone(),
                binding_for(caller.window_id().as_str()),
                caller.session_lifetime(),
            ),
        ) {
            Ok(receipt) => SurfaceSessionResponse::Started {
                session: TransferSessionStarted::from_domain(request_id, receipt),
            },
            Err(error) => SurfaceSessionResponse::Aborted {
                abort: SurfaceTransferAbort::from_domain(request_id, &error),
            },
        }
    }

    fn commit_surface(
        &mut self,
        coordinator: &mut TransferCoordinator,
        _caller: TransferCallerAuthority,
        request: SurfaceTransferCommand,
        selector: TargetSelector,
        live_windows: Vec<longhorn_transfer::LiveTransferWindow>,
    ) -> SurfaceTransferResponse {
        let request_id = request.request_id().clone();
        match commit_surface_transfer(
            self.domains.store(),
            self.domains.surface(),
            self.domains.layout().registry(),
            coordinator,
            &self.clock,
            &self.bindings,
            &self.policy,
            &mut self.provisioner,
            SurfaceTransferCommitRequest::new(
                request.session_id(),
                selector,
                live_windows,
                mutation_options(),
            ),
        ) {
            Ok(receipt) => SurfaceTransferResponse::Committed {
                completion: Box::new(SurfaceTransferCompletion::from_domain(request_id, &receipt)),
            },
            Err(error) => SurfaceTransferResponse::Aborted {
                abort: SurfaceTransferAbort::from_domain(request_id, &error),
            },
        }
    }
}

struct ProofProvisioner {
    app: AppHandle<Wry>,
    host: Arc<TauriWindowHost<Wry>>,
    generation: u64,
}

impl ProofProvisioner {
    const fn new(app: AppHandle<Wry>, host: Arc<TauriWindowHost<Wry>>) -> Self {
        Self {
            app,
            host,
            generation: 0,
        }
    }

    fn apply_set(
        &mut self,
        provisioned: Option<WindowPlacement>,
    ) -> Result<(), SurfaceWindowProvisionFailure> {
        self.generation = self.generation.checked_add(1).ok_or_else(|| {
            failure(
                SurfaceWindowProvisionStage::CreateHidden,
                "proof apply generation exhausted",
            )
        })?;
        let managed = self.host.managed_windows().map_err(|error| {
            failure(
                SurfaceWindowProvisionStage::CreateHidden,
                format!("{error:?}"),
            )
        })?;
        let observation = observe_tauri_desktop(
            &self.app,
            &managed,
            &mut DefaultDisplayMetadata,
            &UniformScaleMapper,
        )
        .map_err(|error| {
            failure(
                SurfaceWindowProvisionStage::CreateHidden,
                format!("{error:?}"),
            )
        })?;
        let mut desired = observation
            .windows()
            .iter()
            .filter_map(|window| {
                window.window_id().cloned().map(|id| {
                    DesiredWindow::new(
                        id,
                        WindowPlacement::new(
                            window.metrics().outer_bounds().origin(),
                            window.metrics().inner_size(),
                        ),
                        window.is_maximized(),
                        window.is_visible(),
                    )
                })
            })
            .collect::<Vec<_>>();
        if let Some(placement) = provisioned {
            desired.push(DesiredWindow::new(
                window_id(PROVISIONED_WINDOW_ID),
                placement,
                false,
                false,
            ));
        }
        let input = WindowDiffInput::new(
            desired,
            observation.windows().iter().cloned(),
            self.host.capabilities(true),
            ApplyGeneration::new(self.generation),
        );
        self.host
            .apply(
                &self.app,
                input,
                provisioned_factory,
                TauriWindowMutationBackend,
                TauriDesktopReadback::new(DefaultDisplayMetadata, UniformScaleMapper),
            )
            .map_err(|error| {
                failure(
                    SurfaceWindowProvisionStage::CreateHidden,
                    format!("{error:?}"),
                )
            })?;
        Ok(())
    }
}

impl SurfaceWindowProvisioner for ProofProvisioner {
    type Authority = WindowId;

    fn provision(
        &mut self,
        request: &SurfaceWindowProvisionRequest,
    ) -> Result<ProvisionedSurfaceWindow<Self::Authority>, SurfaceWindowProvisionFailure> {
        self.apply_set(Some(request.placement()))?;
        let window = self
            .app
            .get_webview_window(PROVISIONED_WINDOW_ID)
            .ok_or_else(|| {
                failure(
                    SurfaceWindowProvisionStage::Ready,
                    "provisioned webview is absent after host apply",
                )
            })?;
        window
            .set_position(LogicalPosition::new(
                f64::from(request.placement().outer_origin().x().get()),
                f64::from(request.placement().outer_origin().y().get()),
            ))
            .map_err(|error| failure(SurfaceWindowProvisionStage::Place, error.to_string()))?;
        window
            .set_size(LogicalSize::new(
                f64::from(request.placement().inner_size().width()),
                f64::from(request.placement().inner_size().height()),
            ))
            .map_err(|error| failure(SurfaceWindowProvisionStage::Place, error.to_string()))?;
        Ok(ProvisionedSurfaceWindow::new(
            request.window_id().clone(),
            SurfaceWindowProvisionReceipt::hidden_ready(
                request.window_id().clone(),
                TransferHostBindingId::new(PROVISIONED_BINDING_ID)
                    .expect("proof binding id is valid"),
                request.display_id().clone(),
                request.placement(),
            ),
        ))
    }

    fn commit(
        &mut self,
        authority: &mut Self::Authority,
    ) -> Result<SurfaceWindowCommitReceipt, SurfaceWindowProvisionFailure> {
        self.app
            .get_webview_window(authority.as_str())
            .ok_or_else(|| {
                failure(
                    SurfaceWindowProvisionStage::Commit,
                    "prepared window disappeared before commit",
                )
            })?
            .show()
            .map_err(|error| failure(SurfaceWindowProvisionStage::Commit, error.to_string()))?;
        Ok(SurfaceWindowCommitReceipt::new(authority.clone()))
    }

    fn cleanup(
        &mut self,
        authority: &mut Self::Authority,
    ) -> Result<SurfaceWindowCleanupReceipt, SurfaceWindowProvisionFailure> {
        self.apply_set(None)
            .map_err(|error| failure(SurfaceWindowProvisionStage::Cleanup, error.to_string()))?;
        Ok(SurfaceWindowCleanupReceipt::new(authority.clone()))
    }
}

pub(crate) fn surface_domain() -> Result<SurfaceDomain, String> {
    RegisteredSurfaceDomain::new(
        DomainDescriptor::new(
            DomainId::new(SURFACE_DOMAIN_ID).expect("proof Surface domain id is valid"),
            SchemaVersion::new(1).expect("proof schema version is valid"),
            StorageClass::MachineState,
            Some(
                DomainFilePath::new("proof/surfaces.json").expect("proof Surface path is portable"),
            ),
        )
        .map_err(|error| error.to_string())?,
        SurfaceDocument::new(
            SurfaceRevision::new(7),
            [
                surface_record(SOURCE_SURFACE_ID, SOURCE_CONTAINER_ID, 0),
                surface_record(SECOND_SURFACE_ID, TARGET_CONTAINER_ID, 1),
            ],
            [],
            [
                ParticipatingWindow::new(
                    window_id(SOURCE_WINDOW_ID),
                    Some(surface_id(SOURCE_SURFACE_ID)),
                ),
                ParticipatingWindow::new(window_id(TARGET_WINDOW_ID), None),
                ParticipatingWindow::new(window_id(PROVISIONED_WINDOW_ID), None),
            ],
        ),
        SurfaceLimits::new(8, 4, 4, 16).expect("proof Surface limits are valid"),
        NoSurfaceMigration,
        SurfaceBackupPolicy::Include,
    )
    .map_err(|error| error.to_string())
}

fn surface_record(id: &str, _container: &str, order: u32) -> SurfaceRecord {
    SurfaceRecord::new(
        surface_id(id),
        LayoutSchemaId::new("schema:proof").expect("proof schema id is valid"),
        Some(id.to_string()),
        [],
        [],
        [
            preference(SOURCE_WINDOW_ID, order),
            preference(TARGET_WINDOW_ID, order),
            preference(PROVISIONED_WINDOW_ID, order),
        ],
    )
}

fn surface_bindings() -> SurfaceHostBindings {
    SurfaceHostBindings::new([
        binding(SOURCE_WINDOW_ID, "binding:source"),
        binding(TARGET_WINDOW_ID, "binding:target"),
        binding(PROVISIONED_WINDOW_ID, PROVISIONED_BINDING_ID),
    ])
    .expect("proof Surface bindings are complete and unique")
}

fn binding(window: &str, id: &str) -> SurfaceHostBinding {
    SurfaceHostBinding::new(
        TransferHostBindingId::new(id).expect("proof binding id is valid"),
        window_id(window),
        DomainId::new(SURFACE_DOMAIN_ID).expect("proof Surface domain id is valid"),
    )
}

fn binding_for(window: &str) -> TransferHostBindingId {
    TransferHostBindingId::new(match window {
        SOURCE_WINDOW_ID => "binding:source",
        TARGET_WINDOW_ID => "binding:target",
        PROVISIONED_WINDOW_ID => PROVISIONED_BINDING_ID,
        _ => "binding:unknown",
    })
    .expect("proof binding ids are valid")
}

fn preference(window: &str, order: u32) -> SurfaceHostPreference {
    SurfaceHostPreference::new(window_id(window), order)
}

fn window_id(value: &str) -> WindowId {
    WindowId::new(value).expect("proof window id is valid")
}

fn surface_id(value: &str) -> SurfaceId {
    SurfaceId::new(value).expect("proof Surface id is valid")
}

fn mutation_options() -> MutationOptions {
    MutationOptions::new(Duration::from_secs(2), DurabilityRequirement::Atomic)
}

fn provisioned_factory(
    app: &AppHandle<Wry>,
    id: &WindowId,
) -> Result<tauri::WebviewWindow<Wry>, longhorn_tauri_windowing::WindowFactoryError> {
    WebviewWindowBuilder::new(app, id.as_str(), WebviewUrl::App("index.html".into()))
        .title("Longhorn Transfer Proof — Provisioned Surface")
        .visible(false)
        .resizable(true)
        .build()
        .map_err(
            |error| longhorn_tauri_windowing::WindowFactoryError::Failed {
                detail: error.to_string(),
            },
        )
}

fn physical_extent(value: u32, scale: longhorn_core::ScaleFactor) -> Result<u32, String> {
    let physical = i32::try_from(value).map_err(|error| error.to_string())?;
    let logical = scale
        .physical_to_screen(PhysicalPx::new(physical), RoundingMode::Nearest)
        .map_err(|error| error.to_string())?
        .get();
    u32::try_from(logical).map_err(|error| error.to_string())
}

fn failure(
    stage: SurfaceWindowProvisionStage,
    detail: impl Into<String>,
) -> SurfaceWindowProvisionFailure {
    SurfaceWindowProvisionFailure::new(stage, detail)
}
