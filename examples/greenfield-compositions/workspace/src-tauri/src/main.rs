use longhorn_command as _;
use longhorn_command_config as _;
use longhorn_command_settings as _;
use longhorn_config as _;
use longhorn_core as _;
use longhorn_display as _;
use longhorn_surfaces_config as _;
use longhorn_settings as _;
use longhorn_settings_config as _;
use longhorn_tauri_command as _;
use longhorn_tauri_config as _;
use longhorn_tauri_settings as _;
use longhorn_tauri_windowing as _;
use longhorn_windowing as _;
use longhorn_windowing_config as _;
use tauri as _;

fn main() {
    println!(
        "{}",
        longhorn_greenfield_proof_common::run(
            "workspace",
            "com.example.longhorn.greenfield.workspace",
        )
    );
}
