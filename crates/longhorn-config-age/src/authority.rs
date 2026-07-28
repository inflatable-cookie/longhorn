use std::{error::Error, fmt, str::FromStr};

use age::secrecy::SecretString;

/// Validated public X25519 age recipient.
#[derive(Clone, Eq, PartialEq)]
pub struct AgeRecipient(pub(crate) age::x25519::Recipient);

impl AgeRecipient {
    /// Parses a public age recipient without retaining the source string.
    pub fn parse(value: &str) -> Result<Self, AgeRecipientError> {
        age::x25519::Recipient::from_str(value)
            .map(Self)
            .map_err(|_| AgeRecipientError)
    }

    /// Returns the canonical public recipient string.
    #[must_use]
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Debug for AgeRecipient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("AgeRecipient")
            .field(&self.as_string())
            .finish()
    }
}

/// Invalid public X25519 recipient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgeRecipientError;

impl fmt::Display for AgeRecipientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid age recipient")
    }
}

impl Error for AgeRecipientError {}

/// Redacted X25519 age identity.
#[derive(Clone)]
pub struct AgeIdentity(pub(crate) age::x25519::Identity);

impl AgeIdentity {
    /// Generates a new X25519 identity from the operating-system RNG.
    #[must_use]
    pub fn generate() -> Self {
        Self(age::x25519::Identity::generate())
    }

    /// Parses a secret identity without retaining or echoing the source string.
    pub fn parse(value: &str) -> Result<Self, AgeIdentityError> {
        age::x25519::Identity::from_str(value)
            .map(Self)
            .map_err(|_| AgeIdentityError)
    }

    /// Derives the public recipient.
    #[must_use]
    pub fn recipient(&self) -> AgeRecipient {
        AgeRecipient(self.0.to_public())
    }

    /// Returns the canonical secret for an explicit secure-store write.
    #[must_use]
    pub fn to_secret(&self) -> SecretString {
        self.0.to_string()
    }
}

impl fmt::Debug for AgeIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AgeIdentity(<redacted>)")
    }
}

/// Invalid secret X25519 identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgeIdentityError;

impl fmt::Display for AgeIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid age identity")
    }
}

impl Error for AgeIdentityError {}

const MAX_PASSPHRASE_BYTES: usize = 4096;

/// Redacted, ephemeral human passphrase for an explicit export or import.
#[derive(Clone)]
pub struct AgePassphrase(pub(crate) SecretString);

impl AgePassphrase {
    /// Accepts a non-empty bounded human passphrase.
    pub fn new(value: String) -> Result<Self, AgePassphraseError> {
        if value.is_empty() {
            Err(AgePassphraseError::Empty)
        } else if value.len() > MAX_PASSPHRASE_BYTES {
            Err(AgePassphraseError::TooLong)
        } else {
            Ok(Self(SecretString::from(value)))
        }
    }
}

impl fmt::Debug for AgePassphrase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AgePassphrase(<redacted>)")
    }
}

/// Invalid explicit passphrase input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgePassphraseError {
    /// Passphrase was empty.
    Empty,
    /// Passphrase exceeded the in-memory input bound.
    TooLong,
}

impl fmt::Display for AgePassphraseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("age passphrase cannot be empty"),
            Self::TooLong => formatter.write_str("age passphrase exceeds the input bound"),
        }
    }
}

impl Error for AgePassphraseError {}

/// Safe failure returned by an injected secure-store or key authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgeProviderError {
    /// Key material is not currently available.
    Unavailable,
    /// Authority would require an interactive prompt.
    InteractionRequired,
    /// Authority failed without exposing provider or secret detail.
    Failed,
}

impl fmt::Display for AgeProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("age key authority is unavailable"),
            Self::InteractionRequired => {
                formatter.write_str("age key authority requires interaction")
            }
            Self::Failed => formatter.write_str("age key authority failed"),
        }
    }
}

impl Error for AgeProviderError {}

/// Injected noninteractive authority for operational backup automation.
pub trait BackupEncryptionProvider {
    /// Loads public recipients for new operational envelopes.
    fn active_recipients(&self) -> Result<Vec<AgeRecipient>, AgeProviderError>;

    /// Loads active and historical identities for inspection and restore.
    fn decryption_identities(&self) -> Result<Vec<AgeIdentity>, AgeProviderError>;
}

/// In-memory active and historical identity ring.
#[derive(Clone)]
pub struct AgeIdentityRing {
    active: AgeIdentity,
    historical: Vec<AgeIdentity>,
}

impl AgeIdentityRing {
    /// Starts a ring with one active identity.
    #[must_use]
    pub fn new(active: AgeIdentity) -> Self {
        Self {
            active,
            historical: Vec::new(),
        }
    }

    /// Adds an older identity retained for existing archives.
    #[must_use]
    pub fn with_historical(mut self, identity: AgeIdentity) -> Self {
        self.historical.push(identity);
        self
    }

    /// Returns the active public recipient.
    #[must_use]
    pub fn active_recipient(&self) -> AgeRecipient {
        self.active.recipient()
    }

    /// Returns the number of available decryption identities.
    #[must_use]
    pub fn identity_count(&self) -> usize {
        1 + self.historical.len()
    }
}

impl fmt::Debug for AgeIdentityRing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgeIdentityRing")
            .field("identity_count", &self.identity_count())
            .finish()
    }
}

impl BackupEncryptionProvider for AgeIdentityRing {
    fn active_recipients(&self) -> Result<Vec<AgeRecipient>, AgeProviderError> {
        Ok(vec![self.active.recipient()])
    }

    fn decryption_identities(&self) -> Result<Vec<AgeIdentity>, AgeProviderError> {
        Ok(std::iter::once(self.active.clone())
            .chain(self.historical.iter().cloned())
            .collect())
    }
}
