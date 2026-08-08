//! Projection helpers.

use super::ForkProjectionError;

pub(crate) fn check_offset(offset: usize, maximum: usize) -> Result<(), ForkProjectionError> {
    if offset > maximum {
        return Err(ForkProjectionError::OffsetOutOfRange {
            maximum,
            actual: offset,
        });
    }
    Ok(())
}

