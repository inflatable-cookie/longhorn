mod error;
mod facts;
mod policy;
mod receipt;
mod request;

pub use error::StorageLayoutError;
pub use facts::{PlatformDirectoryFact, PlatformDirectoryFacts, TargetPlatform};
pub use policy::{
    StorageLayoutWarning, StorageLeafProvenance, StorageProfile, StorageProfileIdError,
    StorageRootProvenance,
};
pub use receipt::{ResolvedStorageLayout, ResolvedStorageRoot, StorageLayoutDiagnostic};
pub use request::{StorageLayoutOverrides, StorageLayoutRequest};

pub(crate) use policy::root_kind_id;
