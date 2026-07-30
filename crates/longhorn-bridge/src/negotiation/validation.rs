use std::collections::HashSet;

use crate::{BridgeNegotiationError, BridgeNegotiationErrorCode};

pub(super) fn validate_limit(
    actual: usize,
    maximum: usize,
    subject: &str,
) -> Result<(), BridgeNegotiationError> {
    if actual > maximum {
        Err(BridgeNegotiationError::new(
            BridgeNegotiationErrorCode::LimitExceeded,
            format!("{subject} count is {actual}; maximum is {maximum}"),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn validate_unique<T>(
    values: &[T],
    code: BridgeNegotiationErrorCode,
    subject: &str,
) -> Result<(), BridgeNegotiationError>
where
    T: Eq + std::hash::Hash,
{
    let mut seen = HashSet::with_capacity(values.len());
    if values.iter().any(|value| !seen.insert(value)) {
        Err(BridgeNegotiationError::new(
            code,
            format!("duplicate {subject}"),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn validate_unique_by<'a, T, K, F>(
    values: &'a [T],
    key: F,
    code: BridgeNegotiationErrorCode,
    subject: &str,
) -> Result<(), BridgeNegotiationError>
where
    K: Eq + std::hash::Hash + 'a,
    F: Fn(&'a T) -> &'a K,
{
    let mut seen = HashSet::with_capacity(values.len());
    if values.iter().map(key).any(|value| !seen.insert(value)) {
        Err(BridgeNegotiationError::new(
            code,
            format!("duplicate {subject}"),
        ))
    } else {
        Ok(())
    }
}
