use longhorn_core::{TransferHostBindingId, WindowId};
use longhorn_surface_transfer::{
    ProvisionedSurfaceWindow, SurfaceWindowCleanupReceipt, SurfaceWindowCommitReceipt,
    SurfaceWindowProvisionFailure, SurfaceWindowProvisionReceipt, SurfaceWindowProvisionRequest,
    SurfaceWindowProvisionStage, SurfaceWindowProvisioner,
};

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
    provisioned_window: Option<WindowId>,
}

impl MockProvisioner {
    pub const fn new(mode: MockMode) -> Self {
        Self {
            mode,
            calls: Vec::new(),
            provisioned_window: None,
        }
    }

    pub fn with_provisioned_window(mut self, window_id: WindowId) -> Self {
        self.provisioned_window = Some(window_id);
        self
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
        let window_id = self
            .provisioned_window
            .as_ref()
            .unwrap_or(request.window_id());
        Ok(ProvisionedSurfaceWindow::new(
            window_id.clone(),
            SurfaceWindowProvisionReceipt::hidden_ready(
                window_id.clone(),
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

pub struct InterveningProvisioner<F> {
    pub inner: MockProvisioner,
    intervene: F,
}

impl<F> InterveningProvisioner<F> {
    pub const fn new(mode: MockMode, intervene: F) -> Self {
        Self {
            inner: MockProvisioner::new(mode),
            intervene,
        }
    }
}

impl<F: FnMut()> SurfaceWindowProvisioner for InterveningProvisioner<F> {
    type Authority = longhorn_core::WindowId;

    fn provision(
        &mut self,
        request: &SurfaceWindowProvisionRequest,
    ) -> Result<ProvisionedSurfaceWindow<Self::Authority>, SurfaceWindowProvisionFailure> {
        let prepared = self.inner.provision(request)?;
        (self.intervene)();
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
