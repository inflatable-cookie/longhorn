//! Storage identity, pure layout resolution, roots, and the bootstrap
//! locator, without the config store.
//!
//! Pure resolution receives platform-directory facts, storage identity,
//! profile selection, and explicit overrides through dependency injection. It
//! performs no filesystem or environment access and returns a complete
//! layout, provenance, warnings, and typed failures. Hosts without Tauri
//! supply facts through [`native_platform_directory_facts`], which reads the
//! process environment once and never invents a missing fact.

mod bootstrap;
mod digest;
mod identity;
mod native;
mod resolver;
mod roots;
mod types;

pub use bootstrap::{
    StorageBootstrapOrigin, StorageBootstrapPathError, StorageBootstrapPaths,
    StorageBootstrapRecovery, StorageBootstrapRecoveryKind, StorageBootstrapSelection,
    StorageBootstrapState, StorageProfileLocator, StorageProfileSelection,
    StorageProfileSelectionError, encode_locator, inspect_storage_bootstrap, locator_matches,
    resolve_storage_bootstrap_paths,
};
pub use digest::StorageLayoutDigest;
pub use identity::{
    StorageIdentity, StorageIdentityError, StorageIdentityErrorKind, StorageIdentityField,
};
pub use native::{
    NativePlatformFactsError, PlatformEnvironment, native_platform_directory_facts,
    native_platform_directory_facts_from,
};
pub use resolver::resolve_storage_layout;
pub use roots::{AccessMode, RootKind, StorageRootError, StorageRoots, root_kind_id};
pub use types::{
    PlatformDirectoryFact, PlatformDirectoryFacts, ResolvedStorageLayout, ResolvedStorageRoot,
    StorageLayoutDiagnostic, StorageLayoutError, StorageLayoutOverrides, StorageLayoutRequest,
    StorageLayoutWarning, StorageLeafProvenance, StorageProfile, StorageProfileIdError,
    StorageRootProvenance, TargetPlatform,
};
