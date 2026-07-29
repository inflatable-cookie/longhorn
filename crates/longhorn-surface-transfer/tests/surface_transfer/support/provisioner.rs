use longhorn_config::ConfigStore;
use longhorn_core::{SurfaceRequestId, SurfaceRevision, TransferHostBindingId};
use longhorn_surface_transfer::{
    ProvisionedSurfaceWindow, SurfaceWindowCleanupReceipt, SurfaceWindowCommitReceipt,
    SurfaceWindowProvisionFailure, SurfaceWindowProvisionReceipt, SurfaceWindowProvisionRequest,
    SurfaceWindowProvisionStage, SurfaceWindowProvisioner,
};
use longhorn_surfaces::{EmptyWindowPolicy, SurfaceMutationCommand, SurfaceMutationRequest};
use longhorn_surfaces_config::publish_surface_mutation;

use super::{TestDomain, layout_document, options, surface_id};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MockMode {
    Success,
    ProvisionFail,
    CleanupFail,
    CommitFail,
}

pub struct MockProvisioner {
    pub mode: MockMode,
    pub calls: Vec<&'static str>,
}

impl MockProvisioner {
    pub const fn new(mode: MockMode) -> Self {
        Self {
            mode,
            calls: Vec::new(),
        }
    }
}

impl SurfaceWindowProvisioner for MockProvisioner {
    type Authority = longhorn_core::WindowId;

    fn provision(
        &mut self,
        request: &SurfaceWindowProvisionRequest,
    ) -> Result<ProvisionedSurfaceWindow<Self::Authority>, SurfaceWindowProvisionFailure> {
        self.calls.push("create_hidden");
        self.calls.push("place");
        self.calls.push("ready");
        if self.mode == MockMode::ProvisionFail {
            return Err(SurfaceWindowProvisionFailure::new(
                SurfaceWindowProvisionStage::Ready,
                "renderer readiness failed",
            ));
        }
        Ok(ProvisionedSurfaceWindow::new(
            request.window_id().clone(),
            SurfaceWindowProvisionReceipt::hidden_ready(
                request.window_id().clone(),
                TransferHostBindingId::new("binding:new").unwrap(),
                request.display_id().clone(),
                request.placement(),
            ),
        ))
    }

    fn commit(
        &mut self,
        authority: &mut Self::Authority,
    ) -> Result<SurfaceWindowCommitReceipt, SurfaceWindowProvisionFailure> {
        self.calls.push("commit");
        if self.mode == MockMode::CommitFail {
            return Err(SurfaceWindowProvisionFailure::new(
                SurfaceWindowProvisionStage::Commit,
                "reveal failed",
            ));
        }
        Ok(SurfaceWindowCommitReceipt::new(authority.clone()))
    }

    fn cleanup(
        &mut self,
        authority: &mut Self::Authority,
    ) -> Result<SurfaceWindowCleanupReceipt, SurfaceWindowProvisionFailure> {
        self.calls.push("cleanup");
        if self.mode == MockMode::CleanupFail {
            return Err(SurfaceWindowProvisionFailure::new(
                SurfaceWindowProvisionStage::Cleanup,
                "close failed",
            ));
        }
        Ok(SurfaceWindowCleanupReceipt::new(authority.clone()))
    }
}

pub struct StalingProvisioner<'a> {
    pub inner: MockProvisioner,
    store: &'a ConfigStore,
    domain: &'a TestDomain,
}

impl<'a> StalingProvisioner<'a> {
    pub const fn new(mode: MockMode, store: &'a ConfigStore, domain: &'a TestDomain) -> Self {
        Self {
            inner: MockProvisioner::new(mode),
            store,
            domain,
        }
    }
}

impl SurfaceWindowProvisioner for StalingProvisioner<'_> {
    type Authority = longhorn_core::WindowId;

    fn provision(
        &mut self,
        request: &SurfaceWindowProvisionRequest,
    ) -> Result<ProvisionedSurfaceWindow<Self::Authority>, SurfaceWindowProvisionFailure> {
        let prepared = self.inner.provision(request)?;
        publish_surface_mutation(
            self.store,
            self.domain,
            options(),
            &layout_document(),
            EmptyWindowPolicy::Allow,
            &SurfaceMutationRequest::new(
                SurfaceRequestId::new("request:intervening").unwrap(),
                SurfaceRevision::new(7),
                SurfaceMutationCommand::RenameSurface {
                    surface_id: surface_id("surface:a"),
                    label: Some("Intervening".to_owned()),
                },
            ),
        )
        .unwrap();
        Ok(prepared)
    }

    fn commit(
        &mut self,
        authority: &mut Self::Authority,
    ) -> Result<SurfaceWindowCommitReceipt, SurfaceWindowProvisionFailure> {
        self.inner.commit(authority)
    }

    fn cleanup(
        &mut self,
        authority: &mut Self::Authority,
    ) -> Result<SurfaceWindowCleanupReceipt, SurfaceWindowProvisionFailure> {
        self.inner.cleanup(authority)
    }
}
