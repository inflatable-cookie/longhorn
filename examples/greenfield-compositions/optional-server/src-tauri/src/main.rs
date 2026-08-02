#[cfg(feature = "server")]
use longhorn_bridge as _;
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
    trace["service"] = if cfg!(feature = "server") {
        "supervised-optional".into()
    } else {
        "absent-local-authority-ready".into()
    };
    println!("{trace}");
}
