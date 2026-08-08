//! Protocol execution over OperationCatalogue.

use crate::OperationCatalogueError;

use super::super::*;

impl From<OperationCatalogueError> for OperationRejection {
    fn from(error: OperationCatalogueError) -> Self {
        let code = match error {
            OperationCatalogueError::AuthorityClosed => OperationRejectionCode::AuthorityClosed,
            OperationCatalogueError::AuthorityMismatch { .. } => {
                OperationRejectionCode::AuthorityMismatch
            }
            OperationCatalogueError::AuthorityEpochMismatch { .. } => {
                OperationRejectionCode::AuthorityEpochMismatch
            }
            OperationCatalogueError::CatalogueRevisionMismatch { .. } => {
                OperationRejectionCode::CatalogueRevisionMismatch
            }
            OperationCatalogueError::DuplicateOperation { .. } => {
                OperationRejectionCode::DuplicateOperation
            }
            OperationCatalogueError::UnknownOperation { .. } => {
                OperationRejectionCode::UnknownOperation
            }
            OperationCatalogueError::InvalidRetrySource { .. } => {
                OperationRejectionCode::InvalidRetrySource
            }
            OperationCatalogueError::InvalidInitialState { .. } => {
                OperationRejectionCode::InvalidInitialState
            }
            OperationCatalogueError::InvalidTransition { .. } => {
                OperationRejectionCode::InvalidTransition
            }
            OperationCatalogueError::OperationRevisionMismatch { .. } => {
                OperationRejectionCode::OperationRevisionMismatch
            }
            OperationCatalogueError::ActiveLimitReached { .. } => {
                OperationRejectionCode::ActiveLimitReached
            }
            OperationCatalogueError::ActiveLimitBelowCurrent { .. } => {
                OperationRejectionCode::ActiveLimitBelowCurrent
            }
            OperationCatalogueError::ProgressNotReportable { .. } => {
                OperationRejectionCode::ProgressNotReportable
            }
            OperationCatalogueError::OverallProgressRegression => {
                OperationRejectionCode::OverallProgressRegression
            }
            OperationCatalogueError::PhaseProgressRegression { .. } => {
                OperationRejectionCode::PhaseProgressRegression
            }
            OperationCatalogueError::DismissalRequiresTerminal { .. } => {
                OperationRejectionCode::DismissalRequiresTerminal
            }
            OperationCatalogueError::DuplicateTeardownResolution { .. } => {
                OperationRejectionCode::DuplicateTeardownResolution
            }
            OperationCatalogueError::MissingTeardownResolutions { .. } => {
                OperationRejectionCode::MissingTeardownResolutions
            }
            OperationCatalogueError::UnexpectedTeardownResolution { .. } => {
                OperationRejectionCode::UnexpectedTeardownResolution
            }
            OperationCatalogueError::InvalidTeardownTerminal { .. } => {
                OperationRejectionCode::InvalidTeardownTerminal
            }
            OperationCatalogueError::TeardownTransferToSelf { .. } => {
                OperationRejectionCode::TeardownTransferToSelf
            }
            OperationCatalogueError::TerminalEncodedWeightOverflow
            | OperationCatalogueError::TerminalEvictionCountOverflow
            | OperationCatalogueError::CatalogueRevisionOverflow
            | OperationCatalogueError::OperationRevisionOverflow
            | OperationCatalogueError::ProgressSequenceOverflow
            | OperationCatalogueError::SequenceOverflow => OperationRejectionCode::CapacityOverflow,
        };
        let refresh_required = matches!(
            code,
            OperationRejectionCode::AuthorityClosed
                | OperationRejectionCode::AuthorityMismatch
                | OperationRejectionCode::AuthorityEpochMismatch
                | OperationRejectionCode::CatalogueRevisionMismatch
                | OperationRejectionCode::UnknownOperation
                | OperationRejectionCode::OperationRevisionMismatch
        );
        Self {
            code,
            detail: error.to_string(),
            refresh_required,
        }
    }
}

impl From<OperationProtocolInputError> for OperationRejection {
    fn from(error: OperationProtocolInputError) -> Self {
        Self {
            code: OperationRejectionCode::InvalidCommand,
            detail: error.to_string(),
            refresh_required: false,
        }
    }
}
