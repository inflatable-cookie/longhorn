use core::fmt;
use std::error::Error;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{LicencePayload, VerifiedLicence};

/// A licence as it is stored and transported.
///
/// The payload travels as **bytes**, not as a parsed structure, and the
/// signature covers exactly those bytes. This is the whole defence against
/// the classic failure in these schemes: verifying a re-serialisation rather
/// than what was received lets any canonicalisation difference — field
/// order, number formatting, whitespace — become a forgery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedLicence {
    /// Which key signed this, so a verifier can select one and rotation can
    /// be reasoned about.
    pub key_id: String,
    /// The exact signed bytes, base64 in the wire form.
    #[serde(with = "base64_bytes")]
    pub payload: Vec<u8>,
    /// Detached Ed25519 signature over `payload`, base64 on the wire.
    #[serde(with = "base64_bytes")]
    pub signature: Vec<u8>,
}

impl SignedLicence {
    /// Records a signed licence.
    #[must_use]
    pub fn new(key_id: impl Into<String>, payload: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            key_id: key_id.into(),
            payload,
            signature,
        }
    }
}

/// Verifies a signed licence against a public key and an expected product.
///
/// The order is deliberate: check the signature over the received bytes
/// **first**, parse them second, and check the product third. Parsing before
/// verifying would run a deserialiser over unauthenticated input; omitting
/// the product check would allow a licence signed for product B to verify for
/// product A whenever products share a signing key.
///
/// The envelope `key_id` is not covered by the signature and is recorded as
/// the claim it is — see `TrustBasis::OfflineSignature`. Verification keys on
/// the caller-supplied key alone.
///
/// # Errors
///
/// Returns [`VerificationError`] when the signature length is invalid, the
/// signature fails verification against `key`, the signed bytes do not parse
/// as a [`LicencePayload`], or the payload's `product` does not match
/// `expected_product`.
pub fn verify(
    signed: &SignedLicence,
    key: &VerifyingKey,
    expected_product: &str,
) -> Result<VerifiedLicence, VerificationError> {
    let signature_bytes: [u8; Signature::BYTE_SIZE] =
        signed.signature.as_slice().try_into().map_err(|_| {
            VerificationError::MalformedSignature {
                expected: Signature::BYTE_SIZE,
                actual: signed.signature.len(),
            }
        })?;
    let signature = Signature::from_bytes(&signature_bytes);

    key.verify_strict(&signed.payload, &signature)
        .map_err(|_| VerificationError::SignatureRejected)?;

    let payload: LicencePayload = serde_json::from_slice(&signed.payload).map_err(|error| {
        VerificationError::MalformedPayload {
            detail: error.to_string(),
        }
    })?;

    if payload.product != expected_product {
        return Err(VerificationError::ProductMismatch {
            expected: expected_product.to_owned(),
            actual: payload.product,
        });
    }

    Ok(VerifiedLicence::from_signature(
        payload,
        signed.key_id.clone(),
    ))
}

/// Licence verification failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerificationError {
    /// The signature was not the right length to be one.
    MalformedSignature {
        /// Required length.
        expected: usize,
        /// Supplied length.
        actual: usize,
    },
    /// The signature did not verify against the key.
    SignatureRejected,
    /// The signature verified but the bytes were not a licence.
    MalformedPayload {
        /// Parser detail.
        detail: String,
    },
    /// The signature verified but the licence was issued for a different product.
    ProductMismatch {
        /// The product the caller expected.
        expected: String,
        /// The product named in the signed payload.
        actual: String,
    },
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedSignature { expected, actual } => write!(
                formatter,
                "signature is {actual} bytes; expected {expected}"
            ),
            Self::SignatureRejected => formatter.write_str("licence signature did not verify"),
            Self::MalformedPayload { detail } => {
                write!(formatter, "signed bytes are not a licence: {detail}")
            }
            Self::ProductMismatch { expected, actual } => {
                write!(
                    formatter,
                    "licence is for `{actual}`, expected `{expected}`"
                )
            }
        }
    }
}

impl Error for VerificationError {}

mod base64_bytes {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(D::Error::custom)
    }
}
