//! Thin binary over the `longhorn-bindings` library.

use std::process::ExitCode;

fn main() -> ExitCode {
    match longhorn_bindings::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("longhorn-bindings: {error}");
            ExitCode::FAILURE
        }
    }
}
