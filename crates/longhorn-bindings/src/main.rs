//! Deterministic checked binding and golden-fixture generation.

use std::{env, error::Error, process::ExitCode};

mod layout;

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

    if arguments.next().is_some() || domain.as_deref() != Some("layout") {
        return Err("usage: longhorn-bindings layout <write|check>".into());
    }

    match mode.as_deref() {
        Some("write") => layout::run(layout::GenerationMode::Write),
        Some("check") => layout::run(layout::GenerationMode::Check),
        _ => Err("usage: longhorn-bindings layout <write|check>".into()),
    }
}
