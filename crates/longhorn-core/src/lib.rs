//! Shared, framework-independent Longhorn primitives.

mod domain_id;
mod schema_version;

pub use domain_id::{DomainId, DomainIdError};
pub use schema_version::{SchemaVersion, SchemaVersionError};
