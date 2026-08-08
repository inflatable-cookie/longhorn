//! Command argument schemas and validation.

mod number;
mod schema;
mod validate;

pub use number::CommandFiniteNumber;
pub use schema::{
    CommandArgumentField, CommandArgumentKind, CommandArgumentSchema, CommandArgumentValue,
    CommandArguments,
};
pub(crate) use validate::{
    invalid_schema, type_mismatch, validate_kind, validate_object, validate_range,
    validate_raw_value, validate_typed_value,
};
