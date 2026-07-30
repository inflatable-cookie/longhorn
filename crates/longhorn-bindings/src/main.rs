//! Deterministic checked binding and golden-fixture generation.

use std::{env, error::Error, process::ExitCode};

mod bridge;
mod commands;
mod config;
mod generation;
mod layout;
mod settings;
mod surface_transfer;
mod surfaces;
mod transfer;

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
        Some("layout") => layout::run(mode),
        Some("settings") => settings::run(mode),
        Some("surfaces") => surfaces::run(mode),
        Some("surface-transfer") => surface_transfer::run(mode),
        Some("transfer") => transfer::run(mode),
        _ => Err(usage().into()),
    }
}

fn usage() -> &'static str {
    "usage: longhorn-bindings \
     <bridge|commands|config|layout|settings|surfaces|surface-transfer|transfer> <write|check>"
}
