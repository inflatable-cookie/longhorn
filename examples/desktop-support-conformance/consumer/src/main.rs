//! A neutral second consumer for Longhorn's desktop support crates.
//!
//! This binary is contract 003 step 6's proof for contracts 024 to 029. It is
//! staged and built outside the Longhorn workspace, from packaged crate
//! sources, and uses only each crate's public API. Every value it passes in is
//! a caller value: a seeded secret, a rule set, a capacity, a bound, a section
//! list, a release query, a program path. No product name, path, catalogue
//! entry or policy appears here.
//!
//! Each unit asserts its contract and panics, with the failing expectation
//! named, when the crates do not agree with it. A clean exit means every unit
//! held.

mod bundle;
#[cfg(unix)]
mod git;
#[cfg(not(unix))]
mod git {
    pub(crate) fn run() -> String {
        panic!("contract 029's real-Git unit runs on Unix hosts only")
    }
}
mod logs;
#[cfg(unix)]
mod pty;
#[cfg(not(unix))]
mod pty {
    pub(crate) fn run() -> String {
        panic!("contract 028's live PTY unit runs on Unix hosts only")
    }
}
mod provision;
mod redact;

use std::process::ExitCode;

/// Runs every unit and prints one line per unit.
fn main() -> ExitCode {
    let units = [
        ("024 longhorn-redact", redact::run()),
        ("025 longhorn-support-log", logs::run()),
        ("026 longhorn-diagnostics", bundle::run()),
        ("027 longhorn-tool-provision", provision::run()),
        ("028 longhorn-pty", pty::run()),
        ("029 longhorn-git-workspace", git::run()),
    ];

    println!("desktop support conformance (contract 003 step 6)");
    for (unit, facts) in &units {
        println!("  {unit}: {facts}");
    }
    println!("outcome: pass");
    ExitCode::SUCCESS
}
