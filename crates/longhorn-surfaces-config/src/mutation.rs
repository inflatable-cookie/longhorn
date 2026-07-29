use std::{error::Error, fmt};

use longhorn_config::{ConfigStore, DomainIssue, MutationError, MutationOptions, MutationReceipt};
use longhorn_layout::LayoutDocument;
use longhorn_surfaces::{
    EmptyWindowPolicy, LayoutContainerInventory, SurfaceMutationEngine, SurfaceMutationReceipt,
    SurfaceMutationRejection, SurfaceMutationRequest,
};

use crate::{RegisteredSurfaceDomain, SurfaceMigration};

/// Successful authoritative Surface and configuration publication evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceConfigPublicationReceipt {
    surface: SurfaceMutationReceipt,
    publication: MutationReceipt,
}

impl SurfaceConfigPublicationReceipt {
    /// Returns the authoritative Surface mutation receipt.
    #[must_use]
    pub const fn surface(&self) -> &SurfaceMutationReceipt {
        &self.surface
    }

    /// Returns the atomic configuration publication receipt.
    #[must_use]
    pub const fn publication(&self) -> &MutationReceipt {
        &self.publication
    }
}

/// Surface mutation rejection or underlying configuration failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceConfigMutationError {
    /// The lifecycle engine rejected the request against exact fresh state.
    Rejected(SurfaceMutationRejection),
    /// Registration, coordination, recovery, validation, or publication failed.
    Config(MutationError),
}

impl fmt::Display for SurfaceConfigMutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(error) => error.fmt(formatter),
            Self::Config(error) => error.fmt(formatter),
        }
    }
}

impl Error for SurfaceConfigMutationError {}

/// Applies one request to fresh coordinated state and immediately publishes it.
pub fn publish_surface_mutation<M>(
    store: &ConfigStore,
    domain: &RegisteredSurfaceDomain<M>,
    options: MutationOptions,
    layout_document: &LayoutDocument,
    empty_window_policy: EmptyWindowPolicy,
    request: &SurfaceMutationRequest,
) -> Result<SurfaceConfigPublicationReceipt, SurfaceConfigMutationError>
where
    M: SurfaceMigration,
{
    let mut surface_receipt = None;
    let mut surface_rejection = None;
    let layout_containers = LayoutContainerInventory::new(
        layout_document
            .containers()
            .iter()
            .map(|container| container.id().clone()),
    );
    let publication = store.mutate(
        domain,
        options,
        |document| match SurfaceMutationEngine::new(
            domain.limits(),
            &layout_containers,
            empty_window_policy,
        )
        .apply(document, request)
        {
            Ok(receipt) => {
                *document = receipt.authoritative_document().clone();
                surface_receipt = Some(receipt);
                Ok(())
            }
            Err(rejection) => {
                let issue = rejection_issue(&rejection);
                surface_rejection = Some(rejection);
                Err(issue)
            }
        },
    );

    match publication {
        Ok(publication) => Ok(SurfaceConfigPublicationReceipt {
            surface: surface_receipt.expect("successful Surface patch records its receipt"),
            publication,
        }),
        Err(MutationError::Patch(_)) if surface_rejection.is_some() => {
            Err(SurfaceConfigMutationError::Rejected(
                surface_rejection.expect("Surface rejection was checked"),
            ))
        }
        Err(error) => Err(SurfaceConfigMutationError::Config(error)),
    }
}

fn rejection_issue(rejection: &SurfaceMutationRejection) -> DomainIssue {
    DomainIssue::new(
        format!("surface-mutation-{:?}", rejection.code()).to_ascii_lowercase(),
        rejection.detail(),
    )
}
