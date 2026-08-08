//! Command argument schemas and validation.

mod number;
mod schema;
mod validate;

pub use number::CommandFiniteNumber;
pub use schema::{
    CommandArgumentField, CommandArgumentKind, CommandArgumentSchema, CommandArgumentValue,
    CommandArguments,
};
pub(crate) use validate::{validate_kind, validate_object, validate_typed_value};
