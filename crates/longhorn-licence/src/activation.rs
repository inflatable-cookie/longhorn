use core::fmt;
use std::error::Error;

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

use crate::{LicenceKey, SignedLicence, Timestamp, VerificationError, VerifiedLicence, verify};

/// A URL an activation request may be issued against.
///
/// HTTPS only. Unlike an update artifact, nothing here is
/// signature-verified end to end by a third party, and an activation request
/// carries credentials.
///
/// Deliberately duplicated rather than shared with `longhorn-update`'s
/// equivalent: both are optional capability crates, and coupling them so one
/// cannot be composed without the other would cost more than thirty lines of
/// validation. Promote to a shared primitive if a third caller appears.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct ActivationUrl(String);

impl From<ActivationUrl> for String {
    fn from(value: ActivationUrl) -> Self {
        value.0
    }
}

impl ActivationUrl {
    /// Validates and records a URL.
    pub fn new(value: impl Into<String>) -> Result<Self, ActivationUrlError> {
        let value = value.into();
        match value.strip_prefix("https://") {
            Some(rest) if !rest.is_empty() => Ok(Self(value)),
            Some(_) => Err(ActivationUrlError::MissingHost),
            None => Err(ActivationUrlError::NotHttps),
        }
    }

    /// Returns the URL.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ActivationUrl {
    type Error = ActivationUrlError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for ActivationUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Activation URL validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationUrlError {
    /// The URL was not HTTPS.
    NotHttps,
    /// No host followed the scheme.
    MissingHost,
}

impl fmt::Display for ActivationUrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotHttps => "activation endpoints must be https",
            Self::MissingHost => "activation endpoint has no host",
        })
    }
}

impl Error for ActivationUrlError {}

/// One HTTP exchange an activation needs, described rather than performed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationRequest {
    /// Where to send it.
    pub url: ActivationUrl,
    /// Body to send, already serialized by the adapter.
    pub body: Vec<u8>,
    /// Headers to send with it.
    pub headers: Vec<(String, String)>,
}

impl ActivationRequest {
    /// Records a request.
    #[must_use]
    pub const fn new(url: ActivationUrl, body: Vec<u8>) -> Self {
        Self {
            url,
            body,
            headers: Vec::new(),
        }
    }

    /// Adds one header.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

/// What an activation operation needs next.
#[derive(Clone, Debug, PartialEq)]
pub enum Activation {
    /// Finished locally; here is the licence.
    Settled(Box<VerifiedLicence>),
    /// Finished locally; nothing further is needed.
    Done,
    /// The host must perform this exchange and return the response to
    /// `accept`.
    Exchange(ActivationRequest),
}

/// What a customer supplies to obtain a licence.
#[derive(Clone, Debug, PartialEq)]
pub enum Credential {
    /// A redemption token typed by the customer.
    Key(LicenceKey),
    /// A bearer token from a completed account sign-in.
    AccountToken(String),
    /// The bytes of a licence file the customer was sent.
    LicenceFile(Vec<u8>),
}

/// Describes how licences are acquired, renewed, and released.
///
/// Adapters describe exchanges; the host performs them. That keeps this
/// crate pure and matches contract 018's `UpdateSource` posture, so a
/// consumer who has integrated the updater recognises the shape.
///
/// An adapter declares its own trust basis honestly and may not present a
/// remote assertion as a signature.
pub trait ActivationSource {
    /// Begins acquiring a licence from a credential.
    fn acquire(&self, credential: &Credential) -> Result<Activation, ActivationError>;

    /// Accepts a response to an exchange this source requested.
    ///
    /// Defaults to refusing, because a source that never requests an
    /// exchange should never be handed a response.
    fn accept(&self, _response: &[u8]) -> Result<VerifiedLicence, ActivationError> {
        Err(ActivationError::UnexpectedResponse)
    }

    /// Begins renewing a licence's lease.
    ///
    /// Defaults to settling with the licence unchanged, which is correct for
    /// any source whose licences carry no lease.
    fn renew(&self, licence: &VerifiedLicence) -> Result<Activation, ActivationError> {
        Ok(Activation::Settled(Box::new(licence.clone())))
    }

    /// Begins releasing an activation slot.
    ///
    /// Self-service release is required by contract 019 and lives in the
    /// interface rather than being left to each consumer: "I got a new
    /// laptop" is the dominant licensing support ticket, and an interface
    /// that cannot express the answer guarantees every one of them reaches a
    /// human.
    ///
    /// Defaults to done, which is correct for a source holding no slot.
    fn release(&self, _licence: &VerifiedLicence) -> Result<Activation, ActivationError> {
        Ok(Activation::Done)
    }
}

/// Activation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivationError {
    /// The credential was not one this source accepts.
    UnsupportedCredential,
    /// A response arrived for an exchange that was never requested.
    UnexpectedResponse,
    /// A composed URL was not usable.
    Url(ActivationUrlError),
    /// The licence did not verify.
    Verification(VerificationError),
    /// The response was not shaped as this source expects.
    MalformedResponse {
        /// Parser detail.
        detail: String,
    },
}

impl From<ActivationUrlError> for ActivationError {
    fn from(value: ActivationUrlError) -> Self {
        Self::Url(value)
    }
}

impl From<VerificationError> for ActivationError {
    fn from(value: VerificationError) -> Self {
        Self::Verification(value)
    }
}

impl fmt::Display for ActivationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCredential => {
                formatter.write_str("this source does not accept that credential")
            }
            Self::UnexpectedResponse => {
                formatter.write_str("a response arrived for no outstanding exchange")
            }
            Self::Url(error) => write!(formatter, "unusable endpoint: {error}"),
            Self::Verification(error) => write!(formatter, "{error}"),
            Self::MalformedResponse { detail } => {
                write!(formatter, "unexpected activation response: {detail}")
            }
        }
    }
}

impl Error for ActivationError {}

/// A licence file the customer was sent, verified locally.
///
/// The honest baseline: no network at runtime at all, and the only shape
/// that serves air-gapped and procurement-heavy customers. It is also the
/// only source that yields an offline-verifiable trust basis, and therefore
/// the only one eligible for full offline grace.
#[derive(Clone, Debug)]
pub struct SignedFileSource {
    key: VerifyingKey,
}

impl SignedFileSource {
    /// Records a source verifying against one public key.
    #[must_use]
    pub const fn new(key: VerifyingKey) -> Self {
        Self { key }
    }
}

impl ActivationSource for SignedFileSource {
    fn acquire(&self, credential: &Credential) -> Result<Activation, ActivationError> {
        let Credential::LicenceFile(bytes) = credential else {
            return Err(ActivationError::UnsupportedCredential);
        };
        let signed: SignedLicence =
            serde_json::from_slice(bytes).map_err(|error| ActivationError::MalformedResponse {
                detail: error.to_string(),
            })?;
        Ok(Activation::Settled(Box::new(verify(&signed, &self.key)?)))
    }
}

/// A short key exchanged with a backend for a licence.
///
/// The backend is the consumer's. This adapter composes the request and
/// verifies the signed licence that comes back; where that licence is
/// produced, and by whom, is outside Longhorn entirely.
#[derive(Clone, Debug)]
pub struct TokenRedemptionSource {
    endpoint: ActivationUrl,
    key: VerifyingKey,
}

impl TokenRedemptionSource {
    /// Records a source redeeming against one endpoint.
    #[must_use]
    pub const fn new(endpoint: ActivationUrl, key: VerifyingKey) -> Self {
        Self { endpoint, key }
    }

    fn exchange(&self, action: &str, payload: &str) -> Activation {
        // Built, not interpolated: the payload is an arbitrary client-boundary
        // string (an account token, a backend-issued activation id), and JSON
        // metacharacters in it must not reshape the request.
        let body = serde_json::json!({ "action": action, "value": payload })
            .to_string()
            .into_bytes();
        Activation::Exchange(
            ActivationRequest::new(self.endpoint.clone(), body)
                .with_header("Content-Type", "application/json"),
        )
    }
}

impl ActivationSource for TokenRedemptionSource {
    fn acquire(&self, credential: &Credential) -> Result<Activation, ActivationError> {
        match credential {
            Credential::Key(key) => Ok(self.exchange("redeem", key.as_str())),
            Credential::AccountToken(token) => Ok(self.exchange("activate", token)),
            Credential::LicenceFile(_) => Err(ActivationError::UnsupportedCredential),
        }
    }

    fn accept(&self, response: &[u8]) -> Result<VerifiedLicence, ActivationError> {
        let signed: SignedLicence = serde_json::from_slice(response).map_err(|error| {
            ActivationError::MalformedResponse {
                detail: error.to_string(),
            }
        })?;
        Ok(verify(&signed, &self.key)?)
    }

    fn renew(&self, licence: &VerifiedLicence) -> Result<Activation, ActivationError> {
        Ok(self.exchange(
            "renew",
            licence
                .payload()
                .activation_id
                .as_deref()
                .unwrap_or_default(),
        ))
    }

    fn release(&self, licence: &VerifiedLicence) -> Result<Activation, ActivationError> {
        Ok(self.exchange(
            "release",
            licence
                .payload()
                .activation_id
                .as_deref()
                .unwrap_or_default(),
        ))
    }
}

/// Records that a remote source asserted a licence at a point in time.
///
/// The bridge for adapters whose backend returns its own response shape
/// rather than a signed licence. Naming it explicitly is the point: a
/// consumer reaching for this is choosing a weaker offline guarantee, and
/// the type says so.
#[must_use]
pub fn asserted_remotely(payload: crate::LicencePayload, checked: Timestamp) -> VerifiedLicence {
    VerifiedLicence::from_remote_assertion(payload, checked)
}
