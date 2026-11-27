use core::fmt;
use std::error::Error;

use ed25519_dalek::VerifyingKey;
use longhorn_url::{EndpointClassificationError, LoopbackHttp, classify_endpoint};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::{LicenceKey, SignedLicence, VerificationError, VerifiedLicence, verify};

/// A URL an activation request may be issued against.
///
/// HTTPS only. Unlike an update artifact, nothing here is
/// signature-verified end to end by a third party, and an activation request
/// carries credentials. Loopback HTTP stays forbidden here even though the
/// shared classifier can allow it for update and browser callers.
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
        match classify_endpoint(&value, LoopbackHttp::Forbidden) {
            Ok(_) => Ok(Self(value)),
            Err(EndpointClassificationError::MissingHost) => Err(ActivationUrlError::MissingHost),
            Err(
                EndpointClassificationError::UnsupportedScheme
                | EndpointClassificationError::InsecureScheme,
            ) => Err(ActivationUrlError::NotHttps),
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
///
/// `Debug` is scrubbed by hand: the body carries the licence key or
/// account token and the headers can carry a bearer token, so neither is
/// ever printed. Diagnostics keep the URL, the body length, and the header
/// names.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
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

impl fmt::Debug for ActivationRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let header_names: Vec<&str> = self.headers.iter().map(|(name, _)| name.as_str()).collect();
        formatter
            .debug_struct("ActivationRequest")
            .field("url", &self.url)
            .field("body_len", &self.body.len())
            .field("headers", &header_names)
            .finish()
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
#[derive(Clone)]
pub enum Credential {
    /// A redemption token typed by the customer.
    Key(LicenceKey),
    /// A bearer token from a completed account sign-in.
    ///
    /// A `SecretString`: zeroized on drop, redacted in `Debug` — the same
    /// discipline config-age applies to its age secrets.
    AccountToken(SecretString),
    /// The bytes of a licence file the customer was sent.
    LicenceFile(Vec<u8>),
}

impl fmt::Debug for Credential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(_) => formatter
                .debug_tuple("Credential::Key")
                .field(&"[REDACTED]")
                .finish(),
            Self::AccountToken(_) => formatter
                .debug_tuple("Credential::AccountToken")
                .field(&"[REDACTED]")
                .finish(),
            Self::LicenceFile(bytes) => formatter
                .debug_struct("Credential::LicenceFile")
                .field("byte_len", &bytes.len())
                .finish(),
        }
    }
}

impl PartialEq for Credential {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Key(left), Self::Key(right)) => crate::account::constant_time_eq(
                left.as_str().as_bytes(),
                right.as_str().as_bytes(),
            ),
            (Self::AccountToken(left), Self::AccountToken(right)) => {
                crate::account::constant_time_eq(
                    left.expose_secret().as_bytes(),
                    right.expose_secret().as_bytes(),
                )
            }
            (Self::LicenceFile(left), Self::LicenceFile(right)) => left == right,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_credential_equality_uses_constant_time_comparison() {
        let key = LicenceKey::from_body(&"0".repeat(crate::MINIMUM_KEY_SYMBOLS - 1)).unwrap();
        let left = Credential::Key(key.clone());
        let right = Credential::Key(key);
        crate::account::COMPARISON_CALLS.with(|calls| {
            let before = calls.get();
            assert_eq!(left, right);
            assert_eq!(calls.get(), before + 1);
        });
    }

    #[test]
    fn debug_redacts_credentials_and_file_contents() {
        let key = LicenceKey::from_body(&"0".repeat(crate::MINIMUM_KEY_SYMBOLS - 1))
            .expect("creates a key for the debug check");
        let key_text = key.as_str().to_owned();
        let key_debug = format!("{:?}", Credential::Key(key));
        let file_contents = String::from_utf8(vec![b'x'; 32]).expect("ASCII bytes are UTF-8");
        let file_bytes = file_contents.as_bytes().to_vec();
        let leaked_bytes = format!("{file_bytes:?}");
        let file_debug = format!("{:?}", Credential::LicenceFile(file_bytes));
        let token = "t".repeat(32);
        let token_debug = format!(
            "{:?}",
            Credential::AccountToken(SecretString::from(token.clone()))
        );

        assert!(!key_debug.contains(&key_text));
        assert!(!file_debug.contains(&leaked_bytes));
        assert!(!token_debug.contains(&token));
        assert!(file_debug.contains("byte_len"));
    }
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
    /// The licence carries no activation slot id, so it cannot be renewed or
    /// released against a backend.
    MissingActivationId,
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
            Self::MissingActivationId => formatter.write_str("licence has no activation id"),
            Self::Url(error) => write!(formatter, "unusable endpoint: {error}"),
            Self::Verification(error) => write!(formatter, "{error}"),
            Self::MalformedResponse { detail } => {
                write!(formatter, "unexpected activation response: {detail}")
            }
        }
    }
}

impl Error for ActivationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Url(error) => Some(error),
            Self::Verification(error) => Some(error),
            Self::UnsupportedCredential
            | Self::UnexpectedResponse
            | Self::MissingActivationId
            | Self::MalformedResponse { .. } => None,
        }
    }
}

/// A licence file the customer was sent, verified locally.
///
/// The honest baseline: no network at runtime at all, and the only shape
/// that serves air-gapped and procurement-heavy customers. It is also the
/// only source that yields an offline-verifiable trust basis, and therefore
/// the only one eligible for full offline grace.
#[derive(Clone, Debug)]
pub struct SignedFileSource {
    key: VerifyingKey,
    product: String,
}

impl SignedFileSource {
    /// Records a source verifying licences for `product` against one public key.
    #[must_use]
    pub fn new(key: VerifyingKey, product: impl Into<String>) -> Self {
        Self {
            key,
            product: product.into(),
        }
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
        Ok(Activation::Settled(Box::new(verify(
            &signed,
            &self.key,
            &self.product,
        )?)))
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
    product: String,
}

impl TokenRedemptionSource {
    /// Records a source redeeming licences for `product` against one endpoint.
    #[must_use]
    pub fn new(endpoint: ActivationUrl, key: VerifyingKey, product: impl Into<String>) -> Self {
        Self {
            endpoint,
            key,
            product: product.into(),
        }
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

    fn activation_id(licence: &VerifiedLicence) -> Result<&str, ActivationError> {
        match licence.payload().activation_id.as_deref() {
            Some(id) if !id.is_empty() => Ok(id),
            _ => Err(ActivationError::MissingActivationId),
        }
    }
}

impl ActivationSource for TokenRedemptionSource {
    fn acquire(&self, credential: &Credential) -> Result<Activation, ActivationError> {
        match credential {
            Credential::Key(key) => Ok(self.exchange("redeem", key.as_str())),
            Credential::AccountToken(token) => Ok(self.exchange("activate", token.expose_secret())),
            Credential::LicenceFile(_) => Err(ActivationError::UnsupportedCredential),
        }
    }

    fn accept(&self, response: &[u8]) -> Result<VerifiedLicence, ActivationError> {
        let signed: SignedLicence = serde_json::from_slice(response).map_err(|error| {
            ActivationError::MalformedResponse {
                detail: error.to_string(),
            }
        })?;
        Ok(verify(&signed, &self.key, &self.product)?)
    }

    fn renew(&self, licence: &VerifiedLicence) -> Result<Activation, ActivationError> {
        Ok(self.exchange("renew", Self::activation_id(licence)?))
    }

    fn release(&self, licence: &VerifiedLicence) -> Result<Activation, ActivationError> {
        Ok(self.exchange("release", Self::activation_id(licence)?))
    }
}
