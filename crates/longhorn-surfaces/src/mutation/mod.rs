mod engine;
mod error;
mod operation;
mod policy;
mod protocol;

pub use engine::SurfaceMutationEngine;
pub use error::{SurfaceMutationRejection, SurfaceMutationRejectionCode};
pub use policy::EmptyWindowPolicy;
pub use protocol::{
    SurfaceMutationCommand, SurfaceMutationOutcome, SurfaceMutationReceipt, SurfaceMutationRequest,
    SurfaceMutationResponse,
};
