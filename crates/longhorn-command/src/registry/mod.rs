//! Sealed command registry and builder.

mod builder;
mod digest;
mod discovery;
mod sealed;
mod validate;

pub use builder::CommandRegistryBuilder;
pub use discovery::CommandDiscoveryRecord;
pub use sealed::CommandRegistry;
pub(crate) use digest::compute_digest;
pub(crate) use validate::{
    canonicalize_command, check_count, check_unique, insert_unique, validate_commands,
    validate_contexts, validate_limits, validate_text,
};
