//! The capability files are behavior, not packaging: Tauri's ACL gates on
//! the command *name* before dispatch, so stand-in handlers with the real
//! names prove which window may invoke what. The handler bodies are
//! irrelevant — a denied command never reaches one.

use tauri::{WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn page_ready() {}
#[tauri::command]
fn proof_status() {}
#[tauri::command]
fn toggle_maximized() {}
#[tauri::command]
fn set_workspace() {}
#[tauri::command]
fn prove_protected_primary() {}
#[tauri::command]
fn prepare_missing_display_restart() {}
#[tauri::command]
fn flush_proof() {}
#[tauri::command]
fn quit_proof() {}

fn invoke(label: &str, cmd: &str) -> Result<(), String> {
    let app = tauri::test::mock_builder()
        .invoke_handler(tauri::generate_handler![
            page_ready,
            proof_status,
            toggle_maximized,
            set_workspace,
            prove_protected_primary,
            prepare_missing_display_restart,
            flush_proof,
            quit_proof,
        ])
        .build(tauri::generate_context!())
        .expect("the proof's own context builds");
    let window = WebviewWindowBuilder::new(&app, label, WebviewUrl::default())
        .build()
        .unwrap();
    tauri::test::get_ipc_response(
        &window,
        tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::default(),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[test]
fn the_workspace_window_is_refused_the_drive_commands() {
    for command in ["toggle_maximized", "flush_proof", "quit_proof"] {
        let error = invoke("workspace", command).unwrap_err();
        assert!(
            error.contains("not allowed") || error.contains("denied"),
            "expected an ACL refusal for {command}, found: {error}"
        );
    }
}

#[test]
fn the_workspace_window_keeps_read_and_self_close() {
    invoke("workspace", "page_ready").expect("read passes the ACL");
    invoke("workspace", "set_workspace").expect("self-close passes the ACL");
}

#[test]
fn the_main_window_drives_the_proof() {
    invoke("main", "toggle_maximized").expect("the drive commands pass for main");
    invoke("main", "quit_proof").expect("quit passes for main");
}
