use std::collections::BTreeMap;

use longhorn_core::{DomainId, TransferHostBindingId, WindowId};

use crate::{SurfaceTransferError, SurfaceTransferErrorCode};

/// Current host-owned mapping from one opaque Surface binding to a window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceHostBinding {
    id: TransferHostBindingId,
    window_id: WindowId,
    document_id: DomainId,
}

impl SurfaceHostBinding {
    /// Constructs one current Surface host binding.
    #[must_use]
    pub const fn new(
        id: TransferHostBindingId,
        window_id: WindowId,
        document_id: DomainId,
    ) -> Self {
        Self {
            id,
            window_id,
            document_id,
        }
    }

    /// Returns opaque binding identity.
    #[must_use]
    pub const fn id(&self) -> &TransferHostBindingId {
        &self.id
    }

    /// Returns the current managed window.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the registered Surface domain.
    #[must_use]
    pub const fn document_id(&self) -> &DomainId {
        &self.document_id
    }
}

/// Complete fresh Surface host-binding snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceHostBindings {
    bindings: BTreeMap<TransferHostBindingId, SurfaceHostBinding>,
}

impl SurfaceHostBindings {
    /// Validates unique binding ids and constructs a deterministic snapshot.
    pub fn new(
        bindings: impl IntoIterator<Item = SurfaceHostBinding>,
    ) -> Result<Self, SurfaceTransferError> {
        let mut indexed = BTreeMap::new();
        for binding in bindings {
            let id = binding.id.clone();
            if indexed.insert(id.clone(), binding).is_some() {
                return Err(SurfaceTransferError::new(
                    SurfaceTransferErrorCode::InvalidBindingSnapshot,
                    format!("duplicate Surface host binding {id}"),
                ));
            }
        }
        Ok(Self { bindings: indexed })
    }

    pub(crate) fn get(
        &self,
        id: &TransferHostBindingId,
    ) -> Result<&SurfaceHostBinding, SurfaceTransferError> {
        self.bindings.get(id).ok_or_else(|| {
            SurfaceTransferError::new(
                SurfaceTransferErrorCode::UnknownHostBinding,
                format!("Surface host binding {id} is not current"),
            )
        })
    }
}
