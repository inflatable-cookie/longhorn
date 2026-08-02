use longhorn_config as _;
use longhorn_core as _;
use longhorn_settings as _;
use longhorn_settings_config as _;
use longhorn_tauri_config as _;
use longhorn_tauri_settings as _;
use tauri as _;

fn main() {
    println!(
        "{}",
        longhorn_greenfield_proof_common::run("minimal", "com.example.longhorn.greenfield.minimal",)
    );
}
