//! The Git commit identity derived from the signed-in GitHub account
//! (contract 029, part 5b).
//!
//! The identity is GitHub's noreply address for the account, so a commit made
//! with it is attributable and does not publish a personal address. The
//! resolver caches one identity per account and one account per token digest;
//! it never caches a token.

use std::{collections::HashMap, sync::Mutex, time::SystemTime};

use reqwest::blocking::Client;
use secrecy::{ExposeSecret as _, SecretString};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

use crate::auth::{
    CredentialScope, GithubAuthFailure, TokenClient, build_client, fresh_access_token,
    map_request_error, read_body, require_https,
};

/// One Git author and committer identity, ready for `user.name` and
/// `user.email`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitCommitIdentity {
    /// The name to commit with: the account's display name when it has a usable
    /// one, and the login otherwise.
    pub name: String,
    /// The address to commit with: GitHub's noreply address for the account.
    pub email: String,
}

/// Resolves the commit identity from the stored GitHub grant.
///
/// The application supplies the user-profile endpoint and the user agent; the
/// store, slot, client id and token client arrive per call, as they do for the
/// rest of part 5b.
pub struct GithubCommitIdentityResolver {
    client: Result<Client, GithubAuthFailure>,
    url: String,
    user_agent: String,
    cache: Mutex<CommitIdentityCache>,
}

#[derive(Default)]
struct CommitIdentityCache {
    account_by_token: HashMap<[u8; 32], u64>,
    identity_by_account: HashMap<u64, GitCommitIdentity>,
}

#[derive(Deserialize)]
struct GithubUserProfile {
    id: u64,
    login: String,
    name: Option<String>,
}

impl GithubCommitIdentityResolver {
    /// A resolver for one `https` user-profile endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::NotConfigured`] when the URL is not a
    /// parseable `https` address or the user agent is empty.
    pub fn new(
        user_api_url: impl Into<String>,
        user_agent: impl Into<String>,
    ) -> Result<Self, GithubAuthFailure> {
        let url = user_api_url.into();
        require_https(&url)?;
        Self::with_url(url, user_agent.into())
    }

    /// A resolver for a loopback test server: `http` or `https` on a loopback
    /// host only.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::NotConfigured`] when the URL is not a
    /// loopback address with an explicit port or the user agent is empty.
    #[cfg(feature = "fixture-server")]
    pub fn loopback_for_tests(
        user_api_url: &str,
        user_agent: impl Into<String>,
    ) -> Result<Self, GithubAuthFailure> {
        crate::auth::require_loopback(user_api_url)?;
        Self::with_url(user_api_url.to_owned(), user_agent.into())
    }

    fn with_url(url: String, user_agent: String) -> Result<Self, GithubAuthFailure> {
        if user_agent.trim().is_empty() {
            return Err(GithubAuthFailure::NotConfigured);
        }
        Ok(Self {
            client: build_client(),
            url,
            user_agent,
            cache: Mutex::new(CommitIdentityCache::default()),
        })
    }

    /// Resolves the identity for the stored grant, refreshing it first when it
    /// is inside the refresh skew.
    ///
    /// A cached identity answers without a profile request; the cache is keyed
    /// by the token's digest, so a rotated token is looked up afresh.
    ///
    /// # Errors
    ///
    /// Returns [`GithubAuthFailure::NotSignedIn`] when the slot is empty, the
    /// refresh's own classes when the grant could not be refreshed,
    /// [`GithubAuthFailure::SignInExpired`] when the profile request is
    /// unauthorised, [`GithubAuthFailure::Offline`] on a rate limit or a server
    /// fault, and [`GithubAuthFailure::Failed`] when the profile is unusable.
    pub fn resolve_identity(
        &self,
        scope: &CredentialScope<'_>,
        client_id: &str,
        client: &dyn TokenClient,
        now: SystemTime,
    ) -> Result<GitCommitIdentity, GithubAuthFailure> {
        let token = fresh_access_token(scope, client_id, client, now, false, None)?
            .ok_or(GithubAuthFailure::NotSignedIn)?;
        let fingerprint = fingerprint(&token);
        {
            let cache = self.cache.lock().map_err(|_| GithubAuthFailure::Failed)?;
            if let Some(account_id) = cache.account_by_token.get(&fingerprint)
                && let Some(identity) = cache.identity_by_account.get(account_id)
            {
                return Ok(identity.clone());
            }
        }
        let profile = self.profile(&token)?;
        let login = profile.login.trim();
        if profile.id == 0
            || login.is_empty()
            || !login
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(GithubAuthFailure::Failed);
        }
        let name = profile
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty() && !name.chars().any(char::is_control))
            .unwrap_or(login)
            .to_owned();
        let identity = GitCommitIdentity {
            name,
            email: format!("{}+{}@users.noreply.github.com", profile.id, login),
        };
        let mut cache = self.cache.lock().map_err(|_| GithubAuthFailure::Failed)?;
        cache.account_by_token.insert(fingerprint, profile.id);
        cache
            .identity_by_account
            .insert(profile.id, identity.clone());
        Ok(identity)
    }

    /// Forgets every cached identity. The application calls it on sign-out.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = CommitIdentityCache::default();
        }
    }

    fn profile(&self, token: &SecretString) -> Result<GithubUserProfile, GithubAuthFailure> {
        let client = self.client.as_ref().map_err(Clone::clone)?;
        let response = client
            .get(&self.url)
            .bearer_auth(token.expose_secret())
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", self.user_agent.as_str())
            .send()
            .map_err(map_request_error)?;
        let (status, body) = read_body(response)?;
        if status == 401 {
            return Err(GithubAuthFailure::SignInExpired);
        }
        if status == 429 || (500..600).contains(&status) {
            return Err(GithubAuthFailure::Offline);
        }
        if !(200..300).contains(&status) {
            return Err(GithubAuthFailure::Failed);
        }
        serde_json::from_slice(&body).map_err(|_| GithubAuthFailure::Failed)
    }
}

impl std::fmt::Debug for GithubCommitIdentityResolver {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GithubCommitIdentityResolver")
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

/// The token's digest: the cache key that is never the token.
fn fingerprint(token: &SecretString) -> [u8; 32] {
    Sha256::digest(token.expose_secret().as_bytes()).into()
}
