use core::fmt;
use std::error::Error;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// Shortest verifier RFC 7636 permits.
const MINIMUM_VERIFIER_BYTES: usize = 43;
/// Longest verifier RFC 7636 permits.
const MAXIMUM_VERIFIER_BYTES: usize = 128;

/// A PKCE code verifier.
///
/// RFC 7636. The verifier is held by the application and never leaves it
/// until the token exchange; only its hash travels in the authorization
/// request. That is what stops an attacker who intercepts the redirect from
/// completing the exchange.
///
/// Generating the random value belongs to the host — this crate is pure —
/// but the length and alphabet rules are enforced here so a weak verifier
/// cannot be constructed at all.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeVerifier(String);

impl CodeVerifier {
    /// Validates and records a verifier.
    pub fn new(value: impl Into<String>) -> Result<Self, AccountFlowError> {
        let value = value.into();
        if value.len() < MINIMUM_VERIFIER_BYTES || value.len() > MAXIMUM_VERIFIER_BYTES {
            return Err(AccountFlowError::VerifierLength {
                minimum: MINIMUM_VERIFIER_BYTES,
                maximum: MAXIMUM_VERIFIER_BYTES,
                actual: value.len(),
            });
        }
        if let Some(offending) = value.chars().find(|character| !is_unreserved(*character)) {
            return Err(AccountFlowError::VerifierSymbol { symbol: offending });
        }
        Ok(Self(value))
    }

    /// Returns the verifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the S256 challenge sent in the authorization request.
    ///
    /// S256 only. RFC 7636 also defines `plain`, which sends the verifier
    /// itself and therefore provides no protection at all against an
    /// intercepted redirect; there is no reason to offer it.
    #[must_use]
    pub fn challenge(&self) -> String {
        let digest = Sha256::digest(self.0.as_bytes());
        URL_SAFE_NO_PAD.encode(digest)
    }
}

/// RFC 7636 unreserved characters.
const fn is_unreserved(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_' | '~')
}

/// An in-flight account authorization.
///
/// Holds the two secrets the callback is checked against. Constructed before
/// the browser opens and consumed when it returns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountFlow {
    verifier: CodeVerifier,
    state: String,
    redirect_port: u16,
}

impl AccountFlow {
    /// Begins a flow against a loopback redirect on `redirect_port`.
    ///
    /// `state` must be freshly random per flow; it is what binds the
    /// callback to this request rather than to one an attacker initiated.
    pub fn begin(
        verifier: CodeVerifier,
        state: impl Into<String>,
        redirect_port: u16,
    ) -> Result<Self, AccountFlowError> {
        let state = state.into();
        if state.len() < 16 {
            return Err(AccountFlowError::StateTooShort {
                minimum: 16,
                actual: state.len(),
            });
        }
        Ok(Self {
            verifier,
            state,
            redirect_port,
        })
    }

    /// Returns the loopback redirect URI for this flow.
    ///
    /// RFC 8252: a native application uses the system browser with a
    /// loopback redirect. An embedded webview is not an accepted flow —
    /// it breaks password managers and SSO, and asks the user to type their
    /// credentials into a window the application itself can read.
    #[must_use]
    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.redirect_port)
    }

    /// Returns the S256 challenge for the authorization request.
    #[must_use]
    pub fn challenge(&self) -> String {
        self.verifier.challenge()
    }

    /// Returns the opaque state for the authorization request.
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    /// Checks a callback and yields the authorization code.
    ///
    /// Consumes the flow: a callback may be accepted exactly once, so a
    /// replayed redirect cannot be exchanged a second time.
    pub fn accept_callback(self, callback: &Callback) -> Result<Authorization, AccountFlowError> {
        // State is compared before anything else is read. A callback that
        // did not come from this flow gets nothing, including error detail.
        if !constant_time_eq(callback.state.as_bytes(), self.state.as_bytes()) {
            return Err(AccountFlowError::StateMismatch);
        }
        let redirect_uri = self.redirect_uri();
        match &callback.outcome {
            CallbackOutcome::Code(code) => Ok(Authorization {
                code: code.clone(),
                verifier: self.verifier,
                redirect_uri,
            }),
            CallbackOutcome::Denied { reason } => Err(AccountFlowError::Denied {
                reason: reason.clone(),
            }),
        }
    }
}

/// What the authorization server sent back to the loopback redirect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Callback {
    /// The state echoed back.
    pub state: String,
    /// What happened.
    pub outcome: CallbackOutcome,
}

/// The result carried by a callback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallbackOutcome {
    /// An authorization code to exchange.
    Code(String),
    /// The user or the server declined.
    Denied {
        /// Server-supplied reason, for display.
        reason: String,
    },
}

/// An accepted callback, ready for the token exchange.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Authorization {
    code: String,
    verifier: CodeVerifier,
    redirect_uri: String,
}

impl Authorization {
    /// Returns the authorization code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the verifier to send with the exchange.
    #[must_use]
    pub fn verifier(&self) -> &CodeVerifier {
        &self.verifier
    }

    /// Returns the redirect URI to echo in the exchange.
    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }
}

/// Compares two byte strings without an early exit.
///
/// State comparison is a secret comparison. The timing signal from a naive
/// compare is small, but the mitigation is four lines.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

/// Account authorization failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AccountFlowError {
    /// The verifier was outside the permitted length.
    VerifierLength {
        /// Shortest permitted.
        minimum: usize,
        /// Longest permitted.
        maximum: usize,
        /// Supplied length.
        actual: usize,
    },
    /// The verifier contained a character RFC 7636 does not permit.
    VerifierSymbol {
        /// The offending character.
        symbol: char,
    },
    /// The state value was too short to be unguessable.
    StateTooShort {
        /// Shortest permitted.
        minimum: usize,
        /// Supplied length.
        actual: usize,
    },
    /// The callback did not belong to this flow.
    StateMismatch,
    /// The user or the server declined.
    Denied {
        /// Server-supplied reason.
        reason: String,
    },
}

impl fmt::Display for AccountFlowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VerifierLength {
                minimum,
                maximum,
                actual,
            } => write!(
                formatter,
                "code verifier is {actual} bytes; must be {minimum}-{maximum}"
            ),
            Self::VerifierSymbol { symbol } => {
                write!(formatter, "`{symbol}` is not permitted in a code verifier")
            }
            Self::StateTooShort { minimum, actual } => write!(
                formatter,
                "state is {actual} bytes; at least {minimum} are needed"
            ),
            Self::StateMismatch => formatter.write_str("callback did not match the request"),
            Self::Denied { reason } => write!(formatter, "authorization declined: {reason}"),
        }
    }
}

impl Error for AccountFlowError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn verifier() -> CodeVerifier {
        CodeVerifier::new("a".repeat(64)).unwrap()
    }

    fn flow() -> AccountFlow {
        AccountFlow::begin(verifier(), "state-value-long-enough", 51_234).unwrap()
    }

    #[test]
    fn the_challenge_is_the_rfc_7636_s256_test_vector() {
        // RFC 7636 appendix B. Verifying against the published vector is the
        // only way to know the encoding is right rather than merely
        // self-consistent.
        let verifier = CodeVerifier::new("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();

        assert_eq!(
            verifier.challenge(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn verifier_length_bounds_are_enforced() {
        assert!(matches!(
            CodeVerifier::new("a".repeat(42)),
            Err(AccountFlowError::VerifierLength { actual: 42, .. })
        ));
        assert!(CodeVerifier::new("a".repeat(43)).is_ok());
        assert!(CodeVerifier::new("a".repeat(128)).is_ok());
        assert!(matches!(
            CodeVerifier::new("a".repeat(129)),
            Err(AccountFlowError::VerifierLength { actual: 129, .. })
        ));
    }

    #[test]
    fn a_verifier_outside_the_unreserved_alphabet_is_refused() {
        let mut value = "a".repeat(63);
        value.push('/');

        assert_eq!(
            CodeVerifier::new(value),
            Err(AccountFlowError::VerifierSymbol { symbol: '/' })
        );
    }

    #[test]
    fn the_redirect_is_loopback() {
        assert_eq!(flow().redirect_uri(), "http://127.0.0.1:51234/callback");
    }

    #[test]
    fn a_matching_callback_yields_the_code_and_verifier() {
        let flow = flow();
        let callback = Callback {
            state: flow.state().to_owned(),
            outcome: CallbackOutcome::Code("auth-code".into()),
        };

        let authorization = flow.accept_callback(&callback).unwrap();

        assert_eq!(authorization.code(), "auth-code");
        assert_eq!(authorization.verifier(), &verifier());
        assert_eq!(
            authorization.redirect_uri(),
            "http://127.0.0.1:51234/callback"
        );
    }

    #[test]
    fn a_callback_with_another_state_is_refused() {
        let callback = Callback {
            state: "some-other-state-value".into(),
            outcome: CallbackOutcome::Code("auth-code".into()),
        };

        assert_eq!(
            flow().accept_callback(&callback),
            Err(AccountFlowError::StateMismatch)
        );
    }

    #[test]
    fn state_is_checked_before_the_outcome_is_read() {
        // A denial carrying the wrong state must report the mismatch, not the
        // denial: anything else leaks that a flow was in progress to whoever
        // sent the unmatched callback.
        let callback = Callback {
            state: "wrong-state-but-long".into(),
            outcome: CallbackOutcome::Denied {
                reason: "access_denied".into(),
            },
        };

        assert_eq!(
            flow().accept_callback(&callback),
            Err(AccountFlowError::StateMismatch)
        );
    }

    #[test]
    fn a_denial_with_the_right_state_reports_the_reason() {
        let flow = flow();
        let callback = Callback {
            state: flow.state().to_owned(),
            outcome: CallbackOutcome::Denied {
                reason: "access_denied".into(),
            },
        };

        assert_eq!(
            flow.accept_callback(&callback),
            Err(AccountFlowError::Denied {
                reason: "access_denied".into()
            })
        );
    }

    #[test]
    fn a_short_state_is_refused_at_construction() {
        assert!(matches!(
            AccountFlow::begin(verifier(), "short", 51_234),
            Err(AccountFlowError::StateTooShort { actual: 5, .. })
        ));
    }

    #[test]
    fn constant_time_comparison_still_compares_correctly() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(constant_time_eq(b"", b""));
    }
}
