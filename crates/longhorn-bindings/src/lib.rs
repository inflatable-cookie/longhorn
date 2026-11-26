//! Deterministic checked binding and golden-fixture generation.
//!
//! A mutation rejection carries the exact unchanged authoritative document --
//! that is the protocol's evidence that nothing moved -- and Card 179 made the
//! document larger by folding layout state into it. Boxing the error would
//! change the wire shape to save a stack move on a path this generator only
//! reaches when it deliberately provokes a refusal.
#![allow(clippy::result_large_err)]

use std::{env, error::Error};

mod agent_control;
mod serde_shape;

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

struct Domain {
    name: &'static str,
    render: fn() -> Result<Vec<generation::Artifact>, Box<dyn Error>>,
}

const DOMAINS: &[Domain] = &[
    Domain {
        name: "agent-control",
        render: agent_control::render,
    },
    Domain {
        name: "bridge",
        render: bridge::render,
    },
    Domain {
        name: "commands",
        render: commands::render,
    },
    Domain {
        name: "config",
        render: config::render,
    },
    Domain {
        name: "history",
        render: history::render,
    },
    Domain {
        name: "history-tree",
        render: history_tree::render,
    },
    Domain {
        name: "layout",
        render: layout::render,
    },
    Domain {
        name: "licence",
        render: licence::render,
    },
    Domain {
        name: "native-content",
        render: native_content::render,
    },
    Domain {
        name: "notifications",
        render: notifications::render,
    },
    Domain {
        name: "operation",
        render: operation::render,
    },
    Domain {
        name: "settings",
        render: settings::render,
    },
    Domain {
        name: "surfaces",
        render: surfaces::render,
    },
    Domain {
        name: "surface-transfer",
        render: surface_transfer::render,
    },
    Domain {
        name: "transfer",
        render: transfer::render,
    },
    Domain {
        name: "update",
        render: update::render,
    },
];

/// Runs binding generation for one domain or all domains, parsing
/// `<domain|all> <write|check>` from the process arguments.
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

    let Some(domain) = domain.as_deref() else {
        return Err(usage().into());
    };
    run_domain(domain, mode)
}

/// Runs binding generation for the named domain or `"all"` registered domains.
///
/// # Errors
///
/// Returns an error for an unknown domain, a failed render, or a check-mode
/// diff against the committed tree.
pub fn run_domain(domain: &str, mode: generation::GenerationMode) -> Result<(), Box<dyn Error>> {
    if domain == "all" {
        for entry in DOMAINS {
            let artifacts = (entry.render)()?;
            generation::apply(entry.name, mode, &artifacts)?;
        }
        return Ok(());
    }

    let Some(entry) = DOMAINS.iter().find(|entry| entry.name == domain) else {
        return Err(usage().into());
    };
    let artifacts = (entry.render)()?;
    generation::apply(entry.name, mode, &artifacts)
}

/// Returns the canonical registered domain names in execution order.
pub fn registered_domains() -> impl Iterator<Item = &'static str> {
    DOMAINS.iter().map(|entry| entry.name)
}

fn usage() -> String {
    let names: Vec<&str> = registered_domains().collect();
    format!(
        "usage: longhorn-bindings <all|{}> <write|check>",
        names.join("|")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_lists_all_and_every_registered_domain() {
        let text = usage();
        assert!(text.starts_with("usage: longhorn-bindings <all|"));
        for domain in registered_domains() {
            assert!(text.contains(domain), "missing {domain} in {text}");
        }
    }

    #[test]
    fn run_domain_rejects_unknown_domain_and_all_domains_pass_check() {
        assert!(run_domain("nonexistent", generation::GenerationMode::Check).is_err());
        run_domain("all", generation::GenerationMode::Check).unwrap();
    }
}
