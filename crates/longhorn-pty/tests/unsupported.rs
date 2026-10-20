//! Checks that unsupported platforms expose a classified refusal.

#![cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "illumos"
)))]

use std::collections::BTreeMap;
use std::path::PathBuf;

use longhorn_pty::{CommandSpec, Failure, PtySession, PtySize};

#[test]
fn unsupported_targets_classify_open_as_unsupported() {
    let result = PtySession::open(CommandSpec {
        program: PathBuf::from("unused"),
        arguments: Vec::new(),
        environment: BTreeMap::new(),
        working_directory: PathBuf::from("."),
        size: PtySize {
            rows: 24,
            columns: 80,
        },
    });
    assert!(matches!(result, Err(Failure::Unsupported)));
}
