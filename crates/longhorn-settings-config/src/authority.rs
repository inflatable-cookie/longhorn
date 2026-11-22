use std::sync::Mutex;

use longhorn_core::{DomainId, SettingsAuthorityToken, SettingsScopeId, bytes_to_lowercase_hex};
use longhorn_settings::{
    SettingsAuthorityExpectation, SettingsRegistryGeneration, SettingsScopeRevision,
    SettingsValueProjection,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{SettingsConfigError, SettingsConfigProjection};

pub(crate) struct AuthorityTracker {
    generation: SettingsRegistryGeneration,
    scope_id: SettingsScopeId,
    state: Mutex<AuthorityState>,
}

impl AuthorityTracker {
    pub(crate) const fn new(
        generation: SettingsRegistryGeneration,
        scope_id: SettingsScopeId,
    ) -> Self {
        Self {
            generation,
            scope_id,
            state: Mutex::new(AuthorityState {
                token: None,
                revision: SettingsScopeRevision::INITIAL,
            }),
        }
    }

    pub(crate) fn lock(&self) -> Result<AuthorityGuard<'_>, SettingsConfigError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SettingsConfigError::AuthorityStatePoisoned)?;
        Ok(AuthorityGuard {
            generation: self.generation,
            scope_id: &self.scope_id,
            state,
        })
    }
}

pub(crate) struct AuthorityGuard<'tracker> {
    generation: SettingsRegistryGeneration,
    scope_id: &'tracker SettingsScopeId,
    state: std::sync::MutexGuard<'tracker, AuthorityState>,
}

impl AuthorityGuard<'_> {
    pub(crate) fn observe(
        &mut self,
        token: SettingsAuthorityToken,
    ) -> Result<SettingsAuthorityExpectation, SettingsConfigError> {
        let expectation = self.prepare(token)?;
        self.accept(&expectation);
        Ok(expectation)
    }

    pub(crate) fn prepare(
        &self,
        token: SettingsAuthorityToken,
    ) -> Result<SettingsAuthorityExpectation, SettingsConfigError> {
        let revision = if self
            .state
            .token
            .as_ref()
            .is_some_and(|current| current != &token)
        {
            self.state
                .revision
                .checked_next()
                .map_err(SettingsConfigError::Protocol)?
        } else {
            self.state.revision
        };
        Ok(SettingsAuthorityExpectation {
            registry_generation: self.generation,
            scope_revision: revision,
            authority_token: token,
        })
    }

    pub(crate) fn accept(&mut self, expectation: &SettingsAuthorityExpectation) {
        self.state.revision = expectation.scope_revision;
        self.state.token = Some(expectation.authority_token.clone());
    }

    pub(crate) const fn generation(&self) -> SettingsRegistryGeneration {
        self.generation
    }

    pub(crate) const fn scope_id(&self) -> &SettingsScopeId {
        self.scope_id
    }
}

struct AuthorityState {
    token: Option<SettingsAuthorityToken>,
    revision: SettingsScopeRevision,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadyTokenMaterial<'material> {
    kind: &'static str,
    registry_generation: SettingsRegistryGeneration,
    scope_id: &'material SettingsScopeId,
    domain_id: &'material DomainId,
    encoded_value: &'material serde_json::Value,
    values: &'material [SettingsValueProjection],
}

pub(crate) fn ready_token(
    generation: SettingsRegistryGeneration,
    scope_id: &SettingsScopeId,
    domain_id: &DomainId,
    encoded_value: &serde_json::Value,
    projection: &SettingsConfigProjection,
) -> Result<SettingsAuthorityToken, SettingsConfigError> {
    token(&ReadyTokenMaterial {
        kind: "ready",
        registry_generation: generation,
        scope_id,
        domain_id,
        encoded_value,
        values: projection.values(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryTokenMaterial<'material> {
    kind: &'static str,
    registry_generation: SettingsRegistryGeneration,
    scope_id: &'material SettingsScopeId,
    domain_id: &'material DomainId,
    recovery_code: &'static str,
    source: Option<&'material [u8]>,
}

pub(crate) fn recovery_token(
    generation: SettingsRegistryGeneration,
    scope_id: &SettingsScopeId,
    domain_id: &DomainId,
    recovery_code: &'static str,
    source: Option<&[u8]>,
) -> Result<SettingsAuthorityToken, SettingsConfigError> {
    token(&RecoveryTokenMaterial {
        kind: "recovery",
        registry_generation: generation,
        scope_id,
        domain_id,
        recovery_code,
        source,
    })
}

fn token(material: &impl Serialize) -> Result<SettingsAuthorityToken, SettingsConfigError> {
    let encoded = serde_json::to_vec(material)
        .map_err(|error| SettingsConfigError::AuthorityEncoding(error.to_string()))?;
    let digest = Sha256::digest(encoded);
    let value = format!("sha256:{}", bytes_to_lowercase_hex(&digest));
    SettingsAuthorityToken::new(value)
        .map_err(|error| SettingsConfigError::AuthorityEncoding(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{AuthorityTracker, SettingsConfigError};
    use longhorn_core::{SettingsAuthorityToken, SettingsScopeId};
    use longhorn_settings::{SettingsRegistryGeneration, SettingsScopeRevision};

    #[test]
    fn exhausted_revision_can_observe_current_token_but_cannot_prepare_change() {
        let tracker = AuthorityTracker::new(
            SettingsRegistryGeneration::INITIAL,
            SettingsScopeId::new("test:scope").unwrap(),
        );
        let mut guard = tracker.lock().unwrap();
        let current = SettingsAuthorityToken::new("token:current").unwrap();
        guard.observe(current.clone()).unwrap();
        guard.state.revision = SettingsScopeRevision::new(u64::MAX);

        assert!(matches!(
            guard.prepare(SettingsAuthorityToken::new("token:next").unwrap()),
            Err(SettingsConfigError::Protocol(
                longhorn_settings::SettingsProtocolError::ScopeRevisionOverflow
            ))
        ));
        let unchanged = guard.observe(current.clone()).unwrap();
        assert_eq!(unchanged.scope_revision.get(), u64::MAX);
        assert_eq!(unchanged.authority_token, current);
    }
}
