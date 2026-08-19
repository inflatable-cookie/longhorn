//! Agent-control spike proof app: a Tauri window with a ticking counter,
//! controlled from the outside over a stateless MCP streamable-HTTP server.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod control;
mod server;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("main window must exist");
            server::spawn(window);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("could not run Longhorn agent-control proof");
}
