use std::collections::BTreeMap;

use longhorn_core::{DomainId, LayoutContainerId, TransferHostBindingId, WindowId};
use serde::{Deserialize, Serialize};

use super::{PanelTransferError, PanelTransferErrorCode};

/// Product-neutral shape behind one current panel host binding.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PanelHostBindingKind {
    /// A managed window directly hosts the layout container.
    DirectWindow,
    /// A managed window hosts the layout container through a Surface.
    SurfaceContainer,
}

/// Fresh host-owned mapping from an opaque binding to one layout container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelHostBinding {
    kind: PanelHostBindingKind,
    id: TransferHostBindingId,
    window_id: WindowId,
    document_id: DomainId,
    container_id: LayoutContainerId,
}

impl PanelHostBinding {
    /// Constructs one direct-window binding.
    #[must_use]
    pub const fn direct_window(
        id: TransferHostBindingId,
        window_id: WindowId,
        document_id: DomainId,
        container_id: LayoutContainerId,
    ) -> Self {
        Self::new(
            PanelHostBindingKind::DirectWindow,
            id,
            window_id,
            document_id,
            container_id,
        )
    }

    /// Constructs one Surface-container projection without importing Surface types.
    #[must_use]
    pub const fn surface_container(
        id: TransferHostBindingId,
        window_id: WindowId,
        document_id: DomainId,
        container_id: LayoutContainerId,
    ) -> Self {
        Self::new(
            PanelHostBindingKind::SurfaceContainer,
            id,
            window_id,
            document_id,
            container_id,
        )
    }

    const fn new(
        kind: PanelHostBindingKind,
        id: TransferHostBindingId,
        window_id: WindowId,
        document_id: DomainId,
        container_id: LayoutContainerId,
    ) -> Self {
        Self {
            kind,
            id,
            window_id,
            document_id,
            container_id,
        }
    }

    /// Returns the host composition shape.
    #[must_use]
    pub const fn kind(&self) -> PanelHostBindingKind {
        self.kind
    }

    /// Returns opaque binding identity.
    #[must_use]
    pub const fn id(&self) -> &TransferHostBindingId {
        &self.id
    }

    /// Returns the current managed host window.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns the authoritative registered layout domain.
    #[must_use]
    pub const fn document_id(&self) -> &DomainId {
        &self.document_id
    }

    /// Returns the currently hosted layout container.
    #[must_use]
    pub const fn container_id(&self) -> &LayoutContainerId {
        &self.container_id
    }
}

/// Complete fresh host-binding snapshot used by one admission or commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelHostBindings {
    bindings: BTreeMap<TransferHostBindingId, PanelHostBinding>,
}

impl PanelHostBindings {
    /// Validates unique binding ids and constructs a deterministic snapshot.
    pub fn new(
        bindings: impl IntoIterator<Item = PanelHostBinding>,
    ) -> Result<Self, PanelTransferError> {
        let mut indexed = BTreeMap::new();
        for binding in bindings {
            let id = binding.id.clone();
            if indexed.insert(id.clone(), binding).is_some() {
                return Err(PanelTransferError::new(
                    PanelTransferErrorCode::InvalidBindingSnapshot,
                    format!("duplicate panel host binding {id}"),
                ));
            }
        }
        Ok(Self { bindings: indexed })
    }

    pub(crate) fn get(
        &self,
        id: &TransferHostBindingId,
    ) -> Result<&PanelHostBinding, PanelTransferError> {
        self.bindings.get(id).ok_or_else(|| {
            PanelTransferError::new(
                PanelTransferErrorCode::UnknownHostBinding,
                format!("panel host binding {id} is not current"),
            )
        })
    }
}
