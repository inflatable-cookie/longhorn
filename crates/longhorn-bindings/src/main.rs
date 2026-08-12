//! Deterministic checked binding and golden-fixture generation.
//!
//! A mutation rejection carries the exact unchanged authoritative document --
//! that is the protocol's evidence that nothing moved -- and Card 179 made the
//! document larger by folding layout state into it. Boxing the error would
//! change the wire shape to save a stack move on a path this generator only
//! reaches when it deliberately provokes a refusal.
#![allow(clippy::result_large_err)]

use std::{env, error::Error, process::ExitCode};

mod bridge;
mod commands;
mod config;
mod generation;
mod history;
mod history_tree;
mod layout;
mod native_content;
mod notifications;
mod operation;
mod settings;
mod surface_transfer;
mod surfaces;
mod transfer;
mod update;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("longhorn-bindings: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    let domain = arguments.next();
    let mode = arguments.next();

    if arguments.next().is_some() {
        return Err(usage().into());
    }

    let mode = match mode.as_deref() {
        Some("write") => generation::GenerationMode::Write,
        Some("check") => generation::GenerationMode::Check,
        _ => return Err(usage().into()),
    };
    match domain.as_deref() {
        Some("bridge") => bridge::run(mode),
        Some("commands") => commands::run(mode),
        Some("config") => config::run(mode),
        Some("history") => history::run(mode),
        Some("history-tree") => history_tree::run(mode),
        Some("layout") => layout::run(mode),
        Some("native-content") => native_content::run(mode),
        Some("notifications") => notifications::run(mode),
        Some("update") => update::run(mode),
        Some("operation") => operation::run(mode),
        Some("settings") => settings::run(mode),
        Some("surfaces") => surfaces::run(mode),
        Some("surface-transfer") => surface_transfer::run(mode),
        Some("transfer") => transfer::run(mode),
        _ => Err(usage().into()),
    }
}

fn usage() -> &'static str {
    "usage: longhorn-bindings \
     <bridge|commands|config|history|history-tree|layout|native-content|notifications|operation|settings|surfaces|surface-transfer|transfer|update> <write|check>"
}
