use std::{error::Error, fmt};

use longhorn_config::{ConfigStore, DomainIssue, MutationError, MutationOptions, MutationReceipt};
use longhorn_layout::{
    LayoutMutationEngine, LayoutMutationReceipt, LayoutMutationRejection, LayoutMutationRequest,
};

use crate::{LayoutMigration, RegisteredLayoutDomain};

/// Successful authoritative layout and configuration publication evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutConfigPublicationReceipt {
    layout: LayoutMutationReceipt,
    publication: MutationReceipt,
}

impl LayoutConfigPublicationReceipt {
    /// Returns the authoritative layout mutation receipt.
    #[must_use]
    pub const fn layout(&self) -> &LayoutMutationReceipt {
        &self.layout
    }

    /// Returns the atomic configuration publication receipt.
    #[must_use]
    pub const fn publication(&self) -> &MutationReceipt {
        &self.publication
    }
}

/// Layout mutation rejection or underlying configuration failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutConfigMutationError {
    /// Card 024 rejected the request against exact fresh state.
    Rejected(LayoutMutationRejection),
    /// Registration, coordination, recovery, validation, or publication failed.
    Config(MutationError),
}

impl fmt::Display for LayoutConfigMutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(error) => error.fmt(formatter),
            Self::Config(error) => error.fmt(formatter),
        }
    }
}

impl Error for LayoutConfigMutationError {}

/// Applies one request to fresh coordinated state and immediately publishes it.
///
/// Structural commands use this path. Presentation commands may also use it
/// when the consumer does not opt into debounce.
pub fn publish_layout_mutation<M>(
    store: &ConfigStore,
    domain: &RegisteredLayoutDomain<M>,
    options: MutationOptions,
    request: &LayoutMutationRequest,
) -> Result<LayoutConfigPublicationReceipt, LayoutConfigMutationError>
where
    M: LayoutMigration,
{
    let mut layout_receipt = None;
    let mut layout_rejection = None;
    let publication = store.mutate(domain, options, |document| match LayoutMutationEngine::new(
        domain.registry(),
    )
    .apply(document, request)
    {
        Ok(receipt) => {
            *document = receipt.authoritative_document().clone();
            layout_receipt = Some(receipt);
            Ok(())
        }
        Err(rejection) => {
            let issue = rejection_issue(&rejection);
            layout_rejection = Some(rejection);
            Err(issue)
        }
    });

    match publication {
        Ok(publication) => Ok(LayoutConfigPublicationReceipt {
            layout: layout_receipt.expect("successful layout patch records its receipt"),
            publication,
        }),
        Err(MutationError::Patch(_)) if layout_rejection.is_some() => {
            Err(LayoutConfigMutationError::Rejected(
                layout_rejection.expect("layout rejection was checked"),
            ))
        }
        Err(error) => Err(LayoutConfigMutationError::Config(error)),
    }
}

pub(crate) fn rejection_issue(rejection: &LayoutMutationRejection) -> DomainIssue {
    DomainIssue::new(
        format!("layout-mutation-{:?}", rejection.code()).to_ascii_lowercase(),
        rejection.detail(),
    )
}
