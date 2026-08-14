use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

/// The longest URL this crate will hand to the operating system.
///
/// Well beyond any authorization endpoint with its query parameters, and far
/// below the point where a platform launcher starts truncating or refusing.
const MAX_BROWSER_URL_BYTES: usize = 2048;

/// A URL that is safe to hand to the operating system's URL launcher.
///
/// **This type is a security boundary, not a convenience.** The URL a native
/// application opens for an RFC 8252 flow is built from a response an
/// authorization server sent, and handing an arbitrary string to a platform
/// launcher is remote code execution with extra steps: `file://` reads local
/// state, a custom scheme reaches whatever application claimed it, and on some
/// platforms a shell metacharacter reaches a shell.
///
/// So this is an allowlist, not a denylist. HTTPS with a host, ASCII, no
/// control characters, no whitespace, no embedded credentials, bounded length.
/// Anything else is refused with a reason.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BrowserUrl(String);

impl BrowserUrl {
    /// Validates a URL for launching.
    pub fn new(value: impl Into<String>) -> Result<Self, BrowserUrlError> {
        let value = value.into();
        if value.len() > MAX_BROWSER_URL_BYTES {
            return Err(BrowserUrlError::TooLong {
                maximum: MAX_BROWSER_URL_BYTES,
                actual: value.len(),
            });
        }
        // Scheme first. Everything below assumes the rest is a URL body, and
        // rejecting `javascript:` or `file:` after parsing a host would be a
        // denylist by another name.
        //
        // HTTPS, with one exception mirrored from `EndpointUrl` in the update
        // domain: plain HTTP is accepted for loopback and nothing else. A
        // packaged proof's stub authorization server lives on 127.0.0.1, and
        // loopback traffic cannot be intercepted off-host. Every rule below
        // -- ASCII, no control characters, no credentials in the authority --
        // still applies to a loopback URL, because the launcher is still
        // handing a string to the operating system.
        let rest = match value.strip_prefix("https://") {
            Some(rest) => rest,
            None => match value.strip_prefix("http://") {
                Some(rest) if is_loopback_host(rest) => rest,
                _ => return Err(BrowserUrlError::NotHttps),
            },
        };

        if let Some((index, character)) = rest
            .char_indices()
            .find(|(_, character)| !character.is_ascii())
        {
            return Err(BrowserUrlError::NonAscii {
                index: index + "https://".len(),
                character,
            });
        }
        if let Some((index, _)) = rest
            .char_indices()
            .find(|(_, character)| character.is_ascii_control() || character.is_ascii_whitespace())
        {
            return Err(BrowserUrlError::ControlOrWhitespace {
                index: index + "https://".len(),
            });
        }

        // The authority ends at the first `/`, `?` or `#`.
        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        if authority.is_empty() {
            return Err(BrowserUrlError::MissingHost);
        }
        // `user:password@host` puts credentials in browser history and in any
        // launcher's argument logging, and it is also how a hostile URL
        // disguises its real host from a reader.
        if authority.contains('@') {
            return Err(BrowserUrlError::EmbeddedCredentials);
        }

        Ok(Self(value))
    }

    /// Returns the validated URL.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BrowserUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for BrowserUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BrowserUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Why a URL was refused for launching.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserUrlError {
    /// The scheme was not `https`.
    NotHttps,
    /// No host followed the scheme.
    MissingHost,
    /// The authority carried `user:password@`.
    EmbeddedCredentials,
    /// A non-ASCII character appeared in the URL.
    NonAscii {
        /// Byte index of the character.
        index: usize,
        /// The character itself.
        character: char,
    },
    /// A control character or whitespace appeared in the URL.
    ControlOrWhitespace {
        /// Byte index of the character.
        index: usize,
    },
    /// The URL exceeded the bounded length.
    TooLong {
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
}

impl fmt::Display for BrowserUrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHttps => formatter.write_str("a launched URL must be https"),
            Self::MissingHost => formatter.write_str("a launched URL has no host"),
            Self::EmbeddedCredentials => {
                formatter.write_str("a launched URL must not carry credentials")
            }
            Self::NonAscii { index, character } => write!(
                formatter,
                "a launched URL has the non-ascii character {character:?} at byte {index}"
            ),
            Self::ControlOrWhitespace { index } => write!(
                formatter,
                "a launched URL has a control character or whitespace at byte {index}"
            ),
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "a launched URL is {actual} bytes; maximum is {maximum}"
            ),
        }
    }
}

impl Error for BrowserUrlError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ordinary_authorization_url_is_accepted() {
        let url = BrowserUrl::new(
            "https://accounts.example.com/authorize?response_type=code&state=abcdef0123456789",
        )
        .unwrap();

        assert!(url.as_str().starts_with("https://accounts.example.com"));
    }

    #[test]
    fn every_scheme_but_https_is_refused() {
        // An allowlist, so these fail for being not-https rather than for
        // being individually recognised as dangerous.
        for value in [
            "http://accounts.example.com/authorize",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "vscode://file/etc/passwd",
            "HTTPS://accounts.example.com",
            "accounts.example.com",
        ] {
            assert_eq!(
                BrowserUrl::new(value),
                Err(BrowserUrlError::NotHttps),
                "{value}"
            );
        }
    }

    #[test]
    fn a_url_with_no_host_is_refused() {
        assert_eq!(
            BrowserUrl::new("https://"),
            Err(BrowserUrlError::MissingHost)
        );
        assert_eq!(
            BrowserUrl::new("https:///path"),
            Err(BrowserUrlError::MissingHost)
        );
    }

    #[test]
    fn embedded_credentials_are_refused() {
        // Both because they leak into history, and because they are how a
        // hostile URL hides its real host from whoever reads it.
        assert_eq!(
            BrowserUrl::new("https://accounts.example.com@evil.test/authorize"),
            Err(BrowserUrlError::EmbeddedCredentials)
        );
        assert_eq!(
            BrowserUrl::new("https://user:secret@accounts.example.com/"),
            Err(BrowserUrlError::EmbeddedCredentials)
        );
    }

    #[test]
    fn control_characters_and_whitespace_are_refused() {
        // A newline is how one argument becomes two in a launcher that reads
        // lines, and a space is how it becomes two in one that splits on them.
        for value in [
            "https://accounts.example.com/a\nb",
            "https://accounts.example.com/a b",
            "https://accounts.example.com/a\tb",
            "https://accounts.example.com/a\0b",
        ] {
            assert!(
                matches!(
                    BrowserUrl::new(value),
                    Err(BrowserUrlError::ControlOrWhitespace { .. })
                ),
                "{value:?}"
            );
        }
    }

    #[test]
    fn non_ascii_is_refused_because_a_reader_cannot_check_it() {
        // Cyrillic "а" in place of ASCII "a". The URL reads correctly to a
        // human and resolves somewhere else.
        assert!(matches!(
            BrowserUrl::new("https://аccounts.example.com/"),
            Err(BrowserUrlError::NonAscii { .. })
        ));
    }

    #[test]
    fn an_over_long_url_is_refused_before_anything_else_is_read() {
        let value = format!("https://example.com/{}", "a".repeat(MAX_BROWSER_URL_BYTES));

        assert!(matches!(
            BrowserUrl::new(value),
            Err(BrowserUrlError::TooLong { .. })
        ));
    }

    #[test]
    fn serde_refuses_the_same_urls_the_constructor_does() {
        assert!(serde_json::from_str::<BrowserUrl>("\"https://example.com/\"").is_ok());
        assert!(serde_json::from_str::<BrowserUrl>("\"file:///etc/passwd\"").is_err());
    }
}

/// Whether an `http://` remainder addresses loopback and only loopback.
///
/// Mirrored from the update domain's `EndpointUrl`, including the IPv6
/// bracket handling: the authority brackets the address, so the port
/// separator cannot be found by splitting on the first colon.
fn is_loopback_host(rest: &str) -> bool {
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = match authority.strip_prefix('[') {
        Some(bracketed) => match bracketed.split_once(']') {
            Some((address, _port)) => address,
            None => return false,
        },
        None => authority.split(':').next().unwrap_or_default(),
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

#[cfg(test)]
mod loopback_tests {
    use super::*;

    /// The one exception: plain HTTP addresses loopback and nothing else.
    #[test]
    fn loopback_http_is_accepted_and_only_loopback() {
        assert!(BrowserUrl::new("http://127.0.0.1:8000/authorize?state=x").is_ok());
        assert!(BrowserUrl::new("http://localhost:8000/authorize").is_ok());
        assert!(BrowserUrl::new("http://[::1]:8000/").is_ok());

        assert!(BrowserUrl::new("http://example.com/").is_err());
        // The lookalikes a hostile URL would try: loopback as a prefix, a
        // suffix, or a userinfo disguise.
        assert!(BrowserUrl::new("http://127.0.0.1.evil.example/").is_err());
        assert!(BrowserUrl::new("http://localhost.evil.example/").is_err());
        assert!(BrowserUrl::new("http://evil.example@127.0.0.1/").is_err());
    }

    /// Every containment rule still applies to a loopback URL.
    #[test]
    fn loopback_http_keeps_the_other_rules() {
        assert!(BrowserUrl::new("http://127.0.0.1/a b").is_err());
        assert!(BrowserUrl::new("http://127.0.0.1/\u{9}tab").is_err());
    }
}
