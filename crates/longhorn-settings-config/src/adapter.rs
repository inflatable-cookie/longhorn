use std::{collections::BTreeSet, error::Error, fmt};

use longhorn_core::SettingsEntryId;
use longhorn_settings::{
    SettingsActivationRequirement, SettingsEditability, SettingsEffectiveSource,
    SettingsMutationOutcome, SettingsMutationTiming, SettingsOpaqueValue, SettingsPolicyEffect,
    SettingsRejection, SettingsSourceDiagnostic, SettingsValueProjection,
};

/// Validated consumer projection for one config-backed scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsConfigProjection {
    values: Vec<SettingsValueProjection>,
    activation_requirements: Vec<SettingsActivationRequirement>,
}

impl SettingsConfigProjection {
    /// Validates and canonicalizes projected values by stable entry identity.
    pub fn new(
        mut values: Vec<SettingsValueProjection>,
        activation_requirements: Vec<SettingsActivationRequirement>,
    ) -> Result<Self, SettingsConfigProjectionError> {
        values.sort_by(|left, right| left.entry_id.cmp(&right.entry_id));
        validate_values(&values)?;
        Ok(Self {
            values,
            activation_requirements,
        })
    }

    /// Returns values in stable entry order.
    #[must_use]
    pub fn values(&self) -> &[SettingsValueProjection] {
        &self.values
    }

    /// Returns outstanding runtime activation state.
    #[must_use]
    pub fn activation_requirements(&self) -> &[SettingsActivationRequirement] {
        &self.activation_requirements
    }

    pub(crate) fn with_activation(
        mut self,
        activation_requirements: Vec<SettingsActivationRequirement>,
    ) -> Self {
        self.activation_requirements = activation_requirements;
        self
    }

    pub(crate) fn add_source_diagnostics(&mut self, diagnostics: &[SettingsSourceDiagnostic]) {
        for value in &mut self.values {
            value.source_diagnostics.extend_from_slice(diagnostics);
        }
    }
}

/// Accepted mutation delivered to activation only after configuration success.
#[derive(Clone, Copy, Debug)]
pub enum SettingsCommittedMutation<'mutation, I> {
    /// A decoded and validated consumer intent was committed.
    Apply(&'mutation I),
    /// The named user overrides were reset.
    Reset(&'mutation [SettingsEntryId]),
}

/// Product-owned projection, intent, validation, patch, reset, and activation.
pub trait SettingsConfigAdapter<T> {
    /// Typed consumer intent decoded from the opaque protocol value.
    type Intent;

    /// Projects configured, effective, default, policy, and editability state.
    fn project(&self, value: &T)
    -> Result<SettingsConfigProjection, SettingsConfigProjectionError>;

    /// Decodes one versioned opaque apply intent.
    fn decode_intent(
        &self,
        intent: &SettingsOpaqueValue,
    ) -> Result<Self::Intent, SettingsRejection>;

    /// Names every projected entry affected by an intent.
    fn targeted_entries(&self, intent: &Self::Intent) -> Vec<SettingsEntryId>;

    /// Performs authoritative product and managed-constraint validation.
    fn validate_intent(
        &self,
        current: &T,
        intent: &Self::Intent,
        projection: &SettingsConfigProjection,
    ) -> Result<(), SettingsRejection>;

    /// Applies one already checked intent to the current typed value.
    fn patch(&self, current: &mut T, intent: &Self::Intent) -> Result<(), SettingsRejection>;

    /// Removes only the named user overrides from the current typed value.
    fn reset(
        &self,
        current: &mut T,
        entry_ids: &[SettingsEntryId],
    ) -> Result<(), SettingsRejection>;

    /// Computes runtime activation after configuration mutation succeeds.
    fn activation_after_commit(
        &self,
        mutation: SettingsCommittedMutation<'_, Self::Intent>,
        timing: SettingsMutationTiming,
        outcome: SettingsMutationOutcome,
        committed: &SettingsConfigProjection,
    ) -> Vec<SettingsActivationRequirement>;
}

/// Invalid or contradictory consumer projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsConfigProjectionError {
    /// Stable consumer-facing category.
    pub code: String,
    /// Diagnostic detail.
    pub detail: String,
}

impl SettingsConfigProjectionError {
    /// Constructs an adapter projection failure.
    #[must_use]
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

impl fmt::Display for SettingsConfigProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl Error for SettingsConfigProjectionError {}

fn validate_values(
    values: &[SettingsValueProjection],
) -> Result<(), SettingsConfigProjectionError> {
    let mut ids = BTreeSet::new();
    for value in values {
        if !ids.insert(&value.entry_id) {
            return Err(SettingsConfigProjectionError::new(
                "duplicate-entry",
                format!("entry {} was projected more than once", value.entry_id),
            ));
        }
        validate_value(value)?;
    }
    Ok(())
}

fn validate_value(value: &SettingsValueProjection) -> Result<(), SettingsConfigProjectionError> {
    if matches!(
        value.policy.as_ref().map(|policy| policy.effect),
        Some(SettingsPolicyEffect::Override)
    ) {
        if value.effective_source != SettingsEffectiveSource::ManagedPolicy {
            return Err(SettingsConfigProjectionError::new(
                "policy-source-mismatch",
                format!("policy override for {} is not effective", value.entry_id),
            ));
        }
        if value.editability == SettingsEditability::Editable {
            return Err(SettingsConfigProjectionError::new(
                "editable-policy-override",
                format!("policy override for {} cannot be editable", value.entry_id),
            ));
        }
    } else if value.effective_source == SettingsEffectiveSource::ManagedPolicy {
        return Err(SettingsConfigProjectionError::new(
            "missing-policy-override",
            format!(
                "entry {} reports managed policy without an override",
                value.entry_id
            ),
        ));
    }

    if value.effective_source == SettingsEffectiveSource::UserConfiguration
        && value.configured.is_none()
    {
        return Err(SettingsConfigProjectionError::new(
            "missing-configured-value",
            format!(
                "entry {} reports user configuration without an override",
                value.entry_id
            ),
        ));
    }
    Ok(())
}
