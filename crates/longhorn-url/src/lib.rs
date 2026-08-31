//! Shared HTTPS endpoint URL classification for Longhorn capability crates.
//!
//! Capability newtypes keep their own public types and policy differences:
//! update allows loopback HTTP for a local shim, activation refuses it because
//! requests carry credentials, and the browser launcher adds an allowlist on
//! top. This crate owns the common scheme split and loopback-host parse so a
//! parsing bug is fixed once.

/// Whether plain HTTP to a loopback host is accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopbackHttp {
    /// Accept `http://` when [`is_loopback_host`] returns true.
    Allowed,
    /// Refuse every non-`https://` URL.
    Forbidden,
}

/// A URL string that passed the shared scheme rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassifiedEndpoint<'a> {
    /// `https://` with a non-empty body after the scheme.
    Https {
        /// Bytes after `https://`.
        rest: &'a str,
    },
    /// Plain `http://` to a loopback host, only when [`LoopbackHttp::Allowed`].
    HttpLoopback {
        /// Bytes after `http://`.
        rest: &'a str,
    },
}

/// Why shared endpoint classification refused a string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointClassificationError {
    /// The scheme was neither HTTPS nor (when allowed) loopback HTTP.
    UnsupportedScheme,
    /// Plain HTTP addressed a non-loopback host.
    InsecureScheme,
    /// `https://` had nothing after the scheme.
    MissingHost,
}

/// Classify a URL string under Longhorn's shared endpoint scheme rules.
///
/// Callers map these outcomes onto their own public error types. This function
/// does not enforce launcher allowlists, credential bans, or length bounds —
/// those stay on the capability newtype that needs them.
pub fn classify_endpoint(
    value: &str,
    loopback_http: LoopbackHttp,
) -> Result<ClassifiedEndpoint<'_>, EndpointClassificationError> {
    if let Some(rest) = value.strip_prefix("https://") {
        if rest.is_empty() {
            return Err(EndpointClassificationError::MissingHost);
        }
        return Ok(ClassifiedEndpoint::Https { rest });
    }
    if matches!(loopback_http, LoopbackHttp::Allowed)
        && let Some(rest) = value.strip_prefix("http://")
    {
        if is_loopback_host(rest) {
            return Ok(ClassifiedEndpoint::HttpLoopback { rest });
        }
        return Err(EndpointClassificationError::InsecureScheme);
    }
    Err(EndpointClassificationError::UnsupportedScheme)
}

/// Whether an `http://` remainder addresses loopback and only loopback.
///
/// The authority ends at the first `/`, `\`, `?`, or `#`. The backslash is
/// not decoration: WHATWG treats it as a path separator for special schemes,
/// so a fetcher built on a conforming URL parser reads
/// `http://evil.example\@127.0.0.1/x` as host `evil.example`. Splitting on
/// `/` alone would hand that string to the userinfo rule and call the
/// authority loopback.
///
/// Userinfo ends at the last `@`. Without stripping it,
/// `127.0.0.1:80@evil.example` parses as host `127.0.0.1` while the fetch
/// goes to `evil.example` over plaintext.
///
/// IPv6 authorities bracket the address, so the port separator cannot be
/// found by splitting on the first colon.
#[must_use]
pub fn is_loopback_host(rest: &str) -> bool {
    let authority = rest.split(['/', '\\', '?', '#']).next().unwrap_or_default();
    let host_part = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = match host_part.strip_prefix('[') {
        Some(bracketed) => match bracketed.split_once(']') {
            Some((address, _port)) => address,
            None => return false,
        },
        None => host_part.split(':').next().unwrap_or_default(),
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_with_a_host_is_accepted() {
        assert_eq!(
            classify_endpoint("https://updates.example.com/x.json", LoopbackHttp::Allowed),
            Ok(ClassifiedEndpoint::Https {
                rest: "updates.example.com/x.json"
            })
        );
        assert_eq!(
            classify_endpoint("https://licences.example.com", LoopbackHttp::Forbidden),
            Ok(ClassifiedEndpoint::Https {
                rest: "licences.example.com"
            })
        );
    }

    #[test]
    fn https_without_a_host_is_missing_host() {
        assert_eq!(
            classify_endpoint("https://", LoopbackHttp::Allowed),
            Err(EndpointClassificationError::MissingHost)
        );
        assert_eq!(
            classify_endpoint("https://", LoopbackHttp::Forbidden),
            Err(EndpointClassificationError::MissingHost)
        );
    }

    #[test]
    fn forbidden_loopback_refuses_every_http_url() {
        for value in [
            "http://127.0.0.1:8000/x",
            "http://licences.example.com",
            "ftp://example.com",
        ] {
            assert_eq!(
                classify_endpoint(value, LoopbackHttp::Forbidden),
                Err(EndpointClassificationError::UnsupportedScheme),
                "{value}"
            );
        }
    }

    #[test]
    fn allowed_loopback_accepts_only_loopback_http() {
        for value in [
            "http://127.0.0.1:8000/production.json",
            "http://localhost:9/x",
            "http://[::1]:80/x",
        ] {
            assert!(
                matches!(
                    classify_endpoint(value, LoopbackHttp::Allowed),
                    Ok(ClassifiedEndpoint::HttpLoopback { .. })
                ),
                "{value}"
            );
        }
        assert_eq!(
            classify_endpoint("http://updates.example.com/x.json", LoopbackHttp::Allowed),
            Err(EndpointClassificationError::InsecureScheme)
        );
        assert_eq!(
            classify_endpoint("ftp://updates.example.com", LoopbackHttp::Allowed),
            Err(EndpointClassificationError::UnsupportedScheme)
        );
    }

    #[test]
    fn userinfo_cannot_make_a_remote_host_look_like_loopback() {
        for value in [
            "http://127.0.0.1:80@evil.example/x",
            "http://[::1]@evil.example/",
            "http://localhost@evil.example/x",
            "http://@evil.example/x",
        ] {
            assert_eq!(
                classify_endpoint(value, LoopbackHttp::Allowed),
                Err(EndpointClassificationError::InsecureScheme),
                "{value}"
            );
        }
    }

    #[test]
    fn a_backslash_cannot_push_the_host_past_the_authority() {
        for value in [
            r"http://evil.example\@127.0.0.1/x",
            r"http://evil.example\@localhost/x",
            r"http://evil.example\@[::1]/x",
            r"http://evil.example\127.0.0.1",
        ] {
            assert_eq!(
                classify_endpoint(value, LoopbackHttp::Allowed),
                Err(EndpointClassificationError::InsecureScheme),
                "{value}"
            );
        }
    }

    #[test]
    fn genuine_loopback_still_passes_with_userinfo_or_backslash_path() {
        for value in [
            "http://user@127.0.0.1:8000/x",
            "http://user:pass@localhost:9/x",
            "http://evil.example@127.0.0.1/x",
            "http://user@[::1]:80/x",
            r"http://127.0.0.1\@evil.example",
            r"http://localhost:9\x",
        ] {
            assert!(
                matches!(
                    classify_endpoint(value, LoopbackHttp::Allowed),
                    Ok(ClassifiedEndpoint::HttpLoopback { .. })
                ),
                "{value}"
            );
        }
    }

    #[test]
    fn a_host_merely_starting_with_localhost_is_not_loopback() {
        assert!(!is_loopback_host("localhost.example.com/x.json"));
        assert!(!is_loopback_host("127.0.0.1.example.com/x.json"));
    }
}
