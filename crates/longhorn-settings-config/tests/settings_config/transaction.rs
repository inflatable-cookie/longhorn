use longhorn_settings::{SettingsApplyCommand, SettingsMutationResult, SettingsResetCommand};
use longhorn_settings_config::{
    ConsumerSettingsTransactionAuthority, ConsumerSettingsTransactionOutcome,
};

struct ExplicitAuthority;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExternalReceipt {
    transaction_id: &'static str,
}

impl ConsumerSettingsTransactionAuthority for ExplicitAuthority {
    type Error = &'static str;
    type Receipt = ExternalReceipt;

    fn apply_transaction(
        &self,
        _command: &SettingsApplyCommand,
    ) -> Result<ConsumerSettingsTransactionOutcome<Self::Receipt>, Self::Error> {
        Err("fixture needs an authorized command")
    }

    fn reset_transaction(
        &self,
        _command: &SettingsResetCommand,
    ) -> Result<ConsumerSettingsTransactionOutcome<Self::Receipt>, Self::Error> {
        Err("fixture needs an authorized command")
    }
}

#[test]
fn broader_transaction_authority_is_explicit_and_separately_receipted() {
    fn receipt_shape<R>(
        result: SettingsMutationResult,
        authority_receipt: R,
    ) -> ConsumerSettingsTransactionOutcome<R> {
        ConsumerSettingsTransactionOutcome {
            result,
            authority_receipt,
        }
    }

    let outcome = receipt_shape(
        SettingsMutationResult::Rejected {
            rejection: longhorn_settings::SettingsRejection {
                code: longhorn_settings::SettingsRejectionCode::Unauthorized,
                diagnostic: None,
            },
            snapshot: None,
        },
        ExternalReceipt {
            transaction_id: "consumer:tx-1",
        },
    );
    assert_eq!(outcome.authority_receipt.transaction_id, "consumer:tx-1");
    let _authority: &dyn ConsumerSettingsTransactionAuthority<Error = &'static str, Receipt = ExternalReceipt> =
        &ExplicitAuthority;
}

#[test]
fn crate_dependency_boundary_excludes_ui_tauri_and_other_domain_systems() {
    let manifest = include_str!("../../Cargo.toml");
    for forbidden in [
        "tauri",
        "longhorn-layout",
        "longhorn-surfaces",
        "longhorn-windowing",
        "svelte",
        "poodle",
    ] {
        assert!(!manifest.contains(forbidden), "found {forbidden}");
    }
}
