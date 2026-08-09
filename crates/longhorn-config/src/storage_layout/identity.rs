use std::{error::Error, fmt};

const MAX_LEAF_BYTES: usize = 255;

/// Field within stable application storage identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageIdentityField {
    /// Canonical application identifier.
    CanonicalApplicationId,
    /// Optional human-readable directory leaf.
    StableStorageName,
}

/// Reason a storage identity field was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageIdentityErrorKind {
    /// Value was empty.
    Empty,
    /// Value exceeded the portable component bound.
    TooLong,
    /// Value was not one safe path component.
    InvalidComponent,
    /// Value is reserved on a supported platform.
    Reserved,
}

/// Invalid canonical application id or explicit storage name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageIdentityError {
    field: StorageIdentityField,
    kind: StorageIdentityErrorKind,
    value: String,
}

impl StorageIdentityError {
    /// Returns the rejected identity field.
    #[must_use]
    pub const fn field(&self) -> StorageIdentityField {
        self.field
    }

    /// Returns the validation failure.
    #[must_use]
    pub const fn kind(&self) -> StorageIdentityErrorKind {
        self.kind
    }

    /// Returns the rejected value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for StorageIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} is {:?}: {:?}",
            self.field, self.kind, self.value
        )
    }
}

impl Error for StorageIdentityError {}

/// Immutable application identity used by storage layout resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageIdentity {
    canonical_application_id: String,
    stable_storage_name: Option<String>,
}

impl StorageIdentity {
    /// Constructs canonical identity with the canonical id as its default leaf.
    pub fn new(canonical_application_id: impl Into<String>) -> Result<Self, StorageIdentityError> {
        let canonical_application_id = canonical_application_id.into();
        validate_canonical_id(&canonical_application_id)?;
        Ok(Self {
            canonical_application_id,
            stable_storage_name: None,
        })
    }

    /// Replaces the canonical leaf with one explicit stable storage name.
    pub fn with_storage_name(
        mut self,
        stable_storage_name: impl Into<String>,
    ) -> Result<Self, StorageIdentityError> {
        let stable_storage_name = stable_storage_name.into();
        validate_leaf(
            StorageIdentityField::StableStorageName,
            &stable_storage_name,
            false,
        )?;
        self.stable_storage_name = Some(stable_storage_name);
        Ok(self)
    }

    /// Returns canonical machine identity.
    #[must_use]
    pub fn canonical_application_id(&self) -> &str {
        &self.canonical_application_id
    }

    /// Returns the optional explicit storage name.
    #[must_use]
    pub fn stable_storage_name(&self) -> Option<&str> {
        self.stable_storage_name.as_deref()
    }

    /// Returns the effective directory leaf.
    #[must_use]
    pub fn effective_leaf(&self) -> &str {
        self.stable_storage_name
            .as_deref()
            .unwrap_or(&self.canonical_application_id)
    }
}

fn validate_canonical_id(value: &str) -> Result<(), StorageIdentityError> {
    validate_leaf(StorageIdentityField::CanonicalApplicationId, value, true)
}

fn validate_leaf(
    field: StorageIdentityField,
    value: &str,
    canonical: bool,
) -> Result<(), StorageIdentityError> {
    let failure = if value.is_empty() {
        Some(StorageIdentityErrorKind::Empty)
    } else if value.len() > MAX_LEAF_BYTES {
        Some(StorageIdentityErrorKind::TooLong)
    } else if matches!(value, "." | "..")
        || value.ends_with(['.', ' '])
        || value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | '\0' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
                )
        })
        || canonical
            && !value.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
            })
    {
        Some(StorageIdentityErrorKind::InvalidComponent)
    } else if is_windows_reserved(value) {
        Some(StorageIdentityErrorKind::Reserved)
    } else {
        None
    };

    match failure {
        Some(kind) => Err(StorageIdentityError {
            field,
            kind,
            value: value.to_owned(),
        }),
        None => Ok(()),
    }
}

fn is_windows_reserved(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                suffix.len() == 1
                    && suffix
                        .as_bytes()
                        .first()
                        .is_some_and(|digit| (b'1'..=b'9').contains(digit))
            })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_id_is_default_and_explicit_name_replaces_it() {
        let canonical = StorageIdentity::new("audio.example.soundcheck").unwrap();
        assert_eq!(canonical.effective_leaf(), "audio.example.soundcheck");

        let named = canonical.with_storage_name("Soundcheck").unwrap();
        assert_eq!(named.effective_leaf(), "Soundcheck");
        assert_eq!(
            named.canonical_application_id(),
            "audio.example.soundcheck"
        );
    }

    #[test]
    fn explicit_invalid_name_never_falls_back() {
        let identity = StorageIdentity::new("audio.example.soundcheck").unwrap();
        let error = identity.with_storage_name("../Soundcheck").unwrap_err();

        assert_eq!(error.field(), StorageIdentityField::StableStorageName);
        assert_eq!(error.kind(), StorageIdentityErrorKind::InvalidComponent);
    }

    #[test]
    fn portable_and_windows_reserved_components_are_rejected() {
        assert_eq!(
            StorageIdentity::new("audio/loophole").unwrap_err().kind(),
            StorageIdentityErrorKind::InvalidComponent
        );
        assert_eq!(
            StorageIdentity::new("CON.json").unwrap_err().kind(),
            StorageIdentityErrorKind::Reserved
        );
        assert_eq!(
            StorageIdentity::new("audio.example.soundcheck")
                .unwrap()
                .with_storage_name("Soundcheck?")
                .unwrap_err()
                .kind(),
            StorageIdentityErrorKind::InvalidComponent
        );
    }
}
