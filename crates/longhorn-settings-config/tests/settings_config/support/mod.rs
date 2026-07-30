mod adapter;
mod domain;
mod fixture;
mod registry;

pub(crate) use adapter::{PolicyMode, PreferencesAdapter, opaque};
pub(crate) use domain::{Preferences, PreferencesDomain};
pub(crate) use fixture::Fixture;
pub(crate) use registry::{
    apply_command, entry_id, load_command, options, reset_command, sealed_registry, unit_id,
};

use longhorn_settings::{SettingsLoadOutcome, SettingsMutationResult, SettingsScopeSnapshot};

pub(crate) fn loaded_snapshot(outcome: SettingsLoadOutcome) -> SettingsScopeSnapshot {
    match outcome {
        SettingsLoadOutcome::Loaded { snapshot } => snapshot,
        SettingsLoadOutcome::Rejected { rejection } => {
            panic!("expected loaded snapshot, found {rejection:?}")
        }
    }
}

pub(crate) fn applied(
    result: SettingsMutationResult,
) -> (
    SettingsScopeSnapshot,
    longhorn_settings::SettingsMutationReceipt,
) {
    match result {
        SettingsMutationResult::Applied { snapshot, receipt } => (snapshot, receipt),
        result => panic!("expected applied settings result, found {result:?}"),
    }
}
