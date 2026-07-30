use longhorn_config::{
    ConfigDomain, LoadDiagnostic, LoadDiagnosticCode, LoadOutcome, MutationError, MutationRefusal,
    RecoveryKind, UnavailableState,
};
use longhorn_settings::{
    SettingsAuthorityExpectation, SettingsProtocolVersion, SettingsRecoveryCode,
    SettingsRecoveryState, SettingsRejectionCode, SettingsScopeSnapshot, SettingsSourceDiagnostic,
};

use crate::{
    ConfigSettingsApplyUnit, SettingsConfigAdapter, SettingsConfigError, SettingsConfigLoadError,
    SettingsConfigProjection,
    authority::{AuthorityGuard, ready_token, recovery_token},
    execution::CommittedProjection,
    validation::rejection,
};

impl<D, A> ConfigSettingsApplyUnit<D, A>
where
    D: ConfigDomain,
    A: SettingsConfigAdapter<D::Value>,
{
    pub(super) fn ready_snapshot(
        &self,
        value: &D::Value,
        diagnostics: &[LoadDiagnostic],
        authority: &mut AuthorityGuard<'_>,
    ) -> Result<SettingsScopeSnapshot, SettingsConfigError> {
        let mut projection = self.adapter.project(value)?;
        projection.add_source_diagnostics(&source_diagnostics(diagnostics));
        let encoded = self
            .domain
            .encode(value)
            .map_err(SettingsConfigLoadError::Encode)
            .map_err(SettingsConfigError::Load)?;
        let token = ready_token(
            authority.generation(),
            authority.scope_id(),
            self.domain.descriptor().id(),
            &encoded,
            &projection,
        )?;
        let expectation = authority.observe(token)?;
        Ok(scope_snapshot(
            &self.definition.scope_id,
            expectation,
            projection,
        ))
    }

    pub(super) fn project_committed(
        &self,
        value: &D::Value,
    ) -> Result<CommittedProjection, SettingsConfigError> {
        let projection = self.adapter.project(value)?;
        let encoded = self
            .domain
            .encode(value)
            .map_err(SettingsConfigLoadError::Encode)
            .map_err(SettingsConfigError::Load)?;
        let token = ready_token(
            self.generation,
            &self.definition.scope_id,
            self.domain.descriptor().id(),
            &encoded,
            &projection,
        )?;
        Ok(CommittedProjection { projection, token })
    }

    pub(super) fn snapshot_from_load(
        &self,
        loaded: LoadOutcome<D::Value>,
        authority: &mut AuthorityGuard<'_>,
    ) -> Result<SettingsScopeSnapshot, SettingsConfigError> {
        match loaded {
            LoadOutcome::Ready(loaded) => {
                self.ready_snapshot(&loaded.value, &loaded.diagnostics, authority)
            }
            LoadOutcome::Recovery(recovery) => self.recovery_snapshot(
                recovery_code(recovery.kind),
                recovery
                    .source
                    .as_ref()
                    .map(|source| source.bytes.as_slice()),
                authority,
            ),
            LoadOutcome::Unavailable(unavailable) => {
                self.recovery_snapshot(unavailable_code(&unavailable), None, authority)
            }
        }
    }

    fn recovery_snapshot(
        &self,
        code: SettingsRecoveryCode,
        source: Option<&[u8]>,
        authority: &mut AuthorityGuard<'_>,
    ) -> Result<SettingsScopeSnapshot, SettingsConfigError> {
        let token = recovery_token(
            authority.generation(),
            authority.scope_id(),
            self.domain.descriptor().id(),
            recovery_code_name(code),
            source,
        )?;
        Ok(SettingsScopeSnapshot {
            protocol_version: SettingsProtocolVersion::CURRENT,
            scope_id: self.definition.scope_id.clone(),
            authority: authority.observe(token)?,
            values: Vec::new(),
            recovery: Some(SettingsRecoveryState {
                code,
                diagnostic: None,
            }),
            activation_requirements: Vec::new(),
        })
    }

    pub(super) fn map_mutation_error(
        &self,
        error: MutationError,
        authority: &mut AuthorityGuard<'_>,
    ) -> Result<longhorn_settings::SettingsMutationResult, SettingsConfigError> {
        match error {
            MutationError::Refused(MutationRefusal::Recovery(recovery)) => {
                let snapshot = self.recovery_snapshot(
                    recovery_code(recovery.kind),
                    recovery
                        .source
                        .as_ref()
                        .map(|source| source.bytes.as_slice()),
                    authority,
                )?;
                Ok(longhorn_settings::SettingsMutationResult::Rejected {
                    rejection: rejection(SettingsRejectionCode::RecoveryRequired),
                    snapshot: Some(snapshot),
                })
            }
            MutationError::Refused(MutationRefusal::Unavailable { location }) => {
                let _ = location;
                let snapshot = self.recovery_snapshot(
                    SettingsRecoveryCode::AuthorityUnavailable,
                    None,
                    authority,
                )?;
                Ok(longhorn_settings::SettingsMutationResult::Rejected {
                    rejection: rejection(SettingsRejectionCode::RecoveryRequired),
                    snapshot: Some(snapshot),
                })
            }
            error => Err(SettingsConfigError::Mutation(error)),
        }
    }
}

pub(super) fn scope_snapshot(
    scope_id: &longhorn_core::SettingsScopeId,
    authority: SettingsAuthorityExpectation,
    projection: SettingsConfigProjection,
) -> SettingsScopeSnapshot {
    SettingsScopeSnapshot {
        protocol_version: SettingsProtocolVersion::CURRENT,
        scope_id: scope_id.clone(),
        authority,
        values: projection.values().to_vec(),
        recovery: None,
        activation_requirements: projection.activation_requirements().to_vec(),
    }
}

fn source_diagnostics(diagnostics: &[LoadDiagnostic]) -> Vec<SettingsSourceDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| SettingsSourceDiagnostic {
            code: match diagnostic.code {
                LoadDiagnosticCode::Missing => "config:missing".to_owned(),
            },
            detail: None,
        })
        .collect()
}

fn recovery_code(kind: RecoveryKind) -> SettingsRecoveryCode {
    match kind {
        RecoveryKind::FutureSchema => SettingsRecoveryCode::FutureSchema,
        RecoveryKind::ReadFailed => SettingsRecoveryCode::AuthorityUnavailable,
        RecoveryKind::CorruptDocument
        | RecoveryKind::DomainMismatch
        | RecoveryKind::InvalidValue
        | RecoveryKind::DecodeFailed => SettingsRecoveryCode::Corrupt,
        RecoveryKind::InvalidDefault
        | RecoveryKind::MissingMigration
        | RecoveryKind::InvalidMigrationStep
        | RecoveryKind::MigrationFailed => SettingsRecoveryCode::RecoveryRequired,
    }
}

fn unavailable_code(unavailable: &UnavailableState) -> SettingsRecoveryCode {
    match unavailable {
        UnavailableState::Authority { .. } => SettingsRecoveryCode::AuthorityUnavailable,
        UnavailableState::RestoreActive => SettingsRecoveryCode::RecoveryInProgress,
        UnavailableState::RestoreRecoveryRequired => SettingsRecoveryCode::RecoveryRequired,
    }
}

const fn recovery_code_name(code: SettingsRecoveryCode) -> &'static str {
    match code {
        SettingsRecoveryCode::Corrupt => "corrupt",
        SettingsRecoveryCode::FutureSchema => "future-schema",
        SettingsRecoveryCode::AuthorityUnavailable => "authority-unavailable",
        SettingsRecoveryCode::RecoveryInProgress => "recovery-in-progress",
        SettingsRecoveryCode::RecoveryRequired => "recovery-required",
    }
}

#[cfg(test)]
mod tests {
    use longhorn_config::UnavailableState;
    use longhorn_settings::SettingsRecoveryCode;

    use super::unavailable_code;

    #[test]
    fn restore_host_gate_stays_distinct_from_general_authority_failure() {
        assert_eq!(
            unavailable_code(&UnavailableState::RestoreActive),
            SettingsRecoveryCode::RecoveryInProgress
        );
        assert_eq!(
            unavailable_code(&UnavailableState::RestoreRecoveryRequired),
            SettingsRecoveryCode::RecoveryRequired
        );
    }
}
