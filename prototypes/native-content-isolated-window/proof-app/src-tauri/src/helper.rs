//! Disposable same-binary Tauri helper with a controlled fake native child.

use std::{
    io::{BufRead, BufReader, Write},
    sync::{Mutex, OnceLock},
    thread,
};

use longhorn_core::PhysicalSize;
use longhorn_native_content_isolated_window_prototype::{
    RuntimeSnapshot, WireCommand, WireCommandKind, WireEvent, WireEventKind,
};
use longhorn_native_content_prototype::AttachGeneration;
use tauri::{Manager, Wry, window::WindowBuilder};

use crate::native_macos;

const WINDOW_LABEL: &str = "isolated-fixture";
static OUTPUT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Copy)]
pub(crate) struct HelperOptions {
    generation: AttachGeneration,
    outer_x: f64,
    outer_y: f64,
    width: f64,
    height: f64,
}

pub(crate) fn options_from_args() -> Result<Option<HelperOptions>, String> {
    let arguments = std::env::args().collect::<Vec<_>>();
    let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--longhorn-isolated-helper")
    else {
        return Ok(None);
    };
    let generation = arguments
        .get(index + 1)
        .ok_or_else(|| "helper generation is missing".to_string())?
        .parse::<u64>()
        .map_err(|error| format!("invalid helper generation: {error}"))?;
    Ok(Some(HelperOptions {
        generation: AttachGeneration::new(generation),
        outer_x: named_f64(&arguments, "--outer-x")?,
        outer_y: named_f64(&arguments, "--outer-y")?,
        width: named_f64(&arguments, "--content-width")?,
        height: named_f64(&arguments, "--content-height")?,
    }))
}

pub(crate) fn run(options: HelperOptions, mut context: tauri::Context<Wry>) {
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .on_window_event(move |_window, event| match event {
            tauri::WindowEvent::Focused(focused) => emit(
                options.generation,
                WireEventKind::FocusChanged { focused: *focused },
            ),
            tauri::WindowEvent::Resized(size) => emit(
                options.generation,
                WireEventKind::ContentResized {
                    size: PhysicalSize::new(size.width, size.height),
                },
            ),
            _ => {}
        })
        .setup(move |app| {
            emit(
                options.generation,
                WireEventKind::Progress {
                    phase: "creating_native_window".to_string(),
                },
            );
            let window = WindowBuilder::new(app, WINDOW_LABEL)
                .title("Longhorn Controlled Native Child")
                .decorations(false)
                .position(options.outer_x, options.outer_y)
                .inner_size(options.width, options.height)
                .visible(false)
                .focused(false)
                .build()?;
            emit(
                options.generation,
                WireEventKind::Progress {
                    phase: "attaching_fake_nsview".to_string(),
                },
            );
            let attached =
                native_macos::install_fake_child(&window).map_err(std::io::Error::other)?;
            let snapshot = snapshot(&window).map_err(std::io::Error::other)?;
            emit(
                options.generation,
                WireEventKind::Ready {
                    snapshot,
                    process_id: std::process::id(),
                    native_child_attached: attached,
                },
            );
            start_command_reader(app.handle().clone(), options.generation);
            Ok(())
        })
        .run(context)
        .expect("could not run isolated-window helper");
}

fn start_command_reader(app: tauri::AppHandle<Wry>, generation: AttachGeneration) {
    thread::spawn(move || {
        for line in BufReader::new(std::io::stdin()).lines() {
            let Ok(line) = line else { break };
            let command = match serde_json::from_str::<WireCommand>(&line) {
                Ok(command) => command,
                Err(_) => continue,
            };
            if matches!(command.command, WireCommandKind::Crash) {
                std::process::exit(73);
            }
            let app_for_command = app.clone();
            if app
                .run_on_main_thread(move || {
                    handle_command(&app_for_command, generation, command);
                })
                .is_err()
            {
                break;
            }
        }
    });
}

fn handle_command(app: &tauri::AppHandle<Wry>, generation: AttachGeneration, command: WireCommand) {
    if command.generation != generation {
        acknowledge(
            generation,
            command.request_id,
            false,
            Some("command generation does not match helper".to_string()),
            None,
        );
        return;
    }
    let Some(window) = app.get_window(WINDOW_LABEL) else {
        acknowledge(
            generation,
            command.request_id,
            false,
            Some("isolated native window is absent".to_string()),
            None,
        );
        return;
    };
    let mut shutdown = false;
    let result = match &command.command {
        WireCommandKind::SetContentSize { size } => window
            .set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
                size.width(),
                size.height(),
            )))
            .map_err(|error| error.to_string()),
        WireCommandKind::Show => window.show().map_err(|error| error.to_string()),
        WireCommandKind::Hide => window.hide().map_err(|error| error.to_string()),
        WireCommandKind::Focus => window.set_focus().map_err(|error| error.to_string()),
        WireCommandKind::ReleaseFocus => native_macos::release_focus(&window),
        WireCommandKind::SetResizable { resizable } => window
            .set_resizable(*resizable)
            .map_err(|error| error.to_string()),
        WireCommandKind::ScriptRequest { request } => {
            emit(
                generation,
                WireEventKind::ChildRequest {
                    request: request.clone(),
                },
            );
            Ok(())
        }
        WireCommandKind::Observe => Ok(()),
        WireCommandKind::Shutdown => {
            shutdown = true;
            Ok(())
        }
        WireCommandKind::Crash => unreachable!("crash is handled before main-thread dispatch"),
    };
    if matches!(
        command.command,
        WireCommandKind::Show | WireCommandKind::Hide
    ) {
        emit(
            generation,
            WireEventKind::VisibilityChanged {
                visible: matches!(command.command, WireCommandKind::Show),
            },
        );
    }
    let fresh = snapshot(&window).ok();
    acknowledge(
        generation,
        command.request_id,
        result.is_ok(),
        result.err(),
        fresh,
    );
    if shutdown {
        emit(generation, WireEventKind::TeardownCompleted);
        let _ = window.close();
        app.exit(0);
    }
}

fn snapshot(window: &tauri::Window<Wry>) -> Result<RuntimeSnapshot, String> {
    let size = window.inner_size().map_err(|error| error.to_string())?;
    Ok(RuntimeSnapshot {
        content_size: PhysicalSize::new(size.width, size.height),
        visible: window.is_visible().map_err(|error| error.to_string())?,
        focused: window.is_focused().map_err(|error| error.to_string())?,
    })
}

fn acknowledge(
    generation: AttachGeneration,
    request_id: u64,
    applied: bool,
    detail: Option<String>,
    snapshot: Option<RuntimeSnapshot>,
) {
    emit(
        generation,
        WireEventKind::Acknowledged {
            request_id,
            applied,
            detail,
            snapshot,
        },
    );
}

fn emit(generation: AttachGeneration, event: WireEventKind) {
    let _guard = OUTPUT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("helper output lock poisoned");
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, &WireEvent { generation, event })
        .expect("helper event must serialize");
    output.write_all(b"\n").expect("helper output must write");
    output.flush().expect("helper output must flush");
}

fn named_f64(arguments: &[String], name: &str) -> Result<f64, String> {
    let index = arguments
        .iter()
        .position(|argument| argument == name)
        .ok_or_else(|| format!("helper argument {name} is missing"))?;
    arguments
        .get(index + 1)
        .ok_or_else(|| format!("helper argument {name} has no value"))?
        .parse::<f64>()
        .map_err(|error| format!("invalid helper argument {name}: {error}"))
}
