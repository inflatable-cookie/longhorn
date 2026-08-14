//! The capability file is behavior, not packaging: Tauri's ACL gates on the
//! command *name* before dispatch, so stand-in handlers with the real names
//! prove which window may invoke what. A window with no matching capability
//! has no IPC access at all.

use tauri::{WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn proof_state() {}
#[tauri::command]
fn open_transfer_session() {}
#[tauri::command]
fn close_transfer_sessions() {}
#[tauri::command]
fn attempt_install() {}
#[tauri::command]
fn attempt_sign_in() {}
#[tauri::command]
fn request_relaunch() {}
#[tauri::command]
fn relaunch_state() {}

fn invoke(label: &str, cmd: &str) -> Result<(), String> {
    let app = tauri::test::mock_builder()
        .invoke_handler(tauri::generate_handler![
            proof_state,
            open_transfer_session,
            close_transfer_sessions,
            attempt_install,
            attempt_sign_in,
            request_relaunch,
            relaunch_state,
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
fn the_main_window_drives_the_proof() {
    invoke("main", "attempt_install").expect("install passes for main");
    invoke("main", "proof_state").expect("read passes for main");
}

#[test]
fn a_window_with_no_capability_has_no_ipc_access() {
    for command in ["proof_state", "attempt_install", "attempt_sign_in"] {
        let error = invoke("secondary", command).unwrap_err();
        assert!(
            error.contains("not allowed") || error.contains("denied"),
            "expected an ACL refusal for {command}, found: {error}"
        );
    }
}
