//! Deterministic checked binding and golden-fixture generation.
//!
//! A mutation rejection carries the exact unchanged authoritative document --
//! that is the protocol's evidence that nothing moved -- and Card 179 made the
//! document larger by folding layout state into it. Boxing the error would
//! change the wire shape to save a stack move on a path this generator only
//! reaches when it deliberately provokes a refusal.
#![allow(clippy::result_large_err)]

use std::{env, error::Error};

/// Bridge protocol bindings and golden fixtures.
pub mod bridge;
/// Command protocol bindings and golden fixtures.
pub mod commands;
/// Config protocol bindings and golden fixtures.
pub mod config;
/// Shared generation primitives: artifact application, tagged-union reads,
/// field maps, and label rendering.
pub mod generation;
/// History protocol bindings and golden fixtures.
pub mod history;
/// Fork-history protocol bindings and golden fixtures.
pub mod history_tree;
/// Layout protocol bindings and golden fixtures.
pub mod layout;
/// Licence protocol bindings and golden fixtures.
pub mod licence;
/// Native-content protocol bindings and golden fixtures.
pub mod native_content;
/// Notification protocol bindings and golden fixtures.
pub mod notifications;
/// Operation protocol bindings and golden fixtures.
pub mod operation;
/// Settings protocol bindings and golden fixtures.
pub mod settings;
/// Surface-transfer protocol bindings and golden fixtures.
pub mod surface_transfer;
/// Surface protocol bindings and golden fixtures.
pub mod surfaces;
/// Transfer protocol bindings and golden fixtures.
pub mod transfer;
/// Update protocol bindings and golden fixtures.
pub mod update;

/// Runs one domain's binding generation, parsing `<domain> <write|check>` from
/// the process arguments.
///
/// # Errors
///
/// Returns an error for unknown arguments, a failed render, or a check-mode
/// diff against the committed tree.
pub fn run() -> Result<(), Box<dyn Error>> {
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
        Some("licence") => licence::run(mode),
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
     <bridge|commands|config|history|history-tree|layout|licence|native-content|notifications|operation|settings|surfaces|surface-transfer|transfer|update> <write|check>"
}
