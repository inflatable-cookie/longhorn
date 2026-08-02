use longhorn_command as _;
use longhorn_command_config as _;
use longhorn_command_settings as _;
use longhorn_config as _;
use longhorn_core as _;
use longhorn_display as _;
use longhorn_history as _;
use longhorn_layout as _;
use longhorn_layout_config as _;
use longhorn_settings as _;
use longhorn_settings_config as _;
use longhorn_surface_transfer as _;
use longhorn_surface_windowing as _;
use longhorn_surfaces as _;
use longhorn_surfaces_config as _;
use longhorn_tauri_command as _;
use longhorn_tauri_config as _;
use longhorn_tauri_history as _;
use longhorn_tauri_settings as _;
use longhorn_tauri_transfer as _;
use longhorn_tauri_windowing as _;
use longhorn_transfer as _;
use longhorn_windowing as _;
use longhorn_windowing_config as _;
use tauri as _;

fn main() {
    println!(
        "{}",
        longhorn_greenfield_proof_common::run(
            "full-hosting",
            "com.example.longhorn.greenfield.full-hosting",
        )
    );
}
