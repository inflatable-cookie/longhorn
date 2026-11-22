#[cfg(feature = "server")]
use longhorn_bridge::{
    BridgeConnectionMachine, BridgeConnectionState, BridgeRetryLimit, BridgeServiceMachine,
    BridgeServiceOwnership, BridgeServiceState,
};
use longhorn_config as _;
use longhorn_core as _;
use longhorn_settings as _;
use longhorn_settings_config as _;
#[cfg(feature = "server")]
use longhorn_tauri_bridge as _;
use longhorn_tauri_config as _;
use longhorn_tauri_settings as _;
use tauri as _;

fn main() {
    let mut trace = longhorn_greenfield_proof_common::run(
        "optional-server",
        "com.example.longhorn.greenfield.optional-server",
    );
    #[cfg(feature = "server")]
    {
        let service = BridgeServiceMachine::new(BridgeServiceOwnership::OwnedLocal);
        let connection = BridgeConnectionMachine::new(
            BridgeRetryLimit::new(2).expect("the example reconnect limit is valid"),
        );
        assert_eq!(service.state(), BridgeServiceState::Absent);
        assert_eq!(connection.status().state(), BridgeConnectionState::Idle);
        trace["service"] = "supervised-optional".into();
    }
    #[cfg(not(feature = "server"))]
    {
        trace["service"] = "absent-local-authority-ready".into();
    }
    println!("{trace}");
}
