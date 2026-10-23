//! Generic worktree status reading.

use std::path::Path;

use crate::{
    execution::Git,
    failure::{GitFailure, GitFailureCode},
    model::GitStatus,
};

impl Git<'_> {
    /// Reads one worktree's staged, modified, untracked and conflicted counts,
    /// its ahead and behind distance from its upstream, and its cleanliness.
    ///
    /// # Errors
    ///
    /// Returns [`GitFailureCode::StatusFailed`] when Git refuses the status
    /// read.
    pub fn status(&self, root: &Path) -> Result<GitStatus, GitFailure> {
        let source = self.run_git(
            root,
            &["status", "--porcelain=v1", "--branch"],
            GitFailureCode::StatusFailed,
        )?;
        Ok(read_status(&source))
    }
}

pub(crate) fn read_status(source: &str) -> GitStatus {
    let mut status = GitStatus {
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        clean: true,
    };
    for line in source.lines() {
        if let Some(header) = line.strip_prefix("## ") {
            status.ahead = branch_distance(header, "ahead");
            status.behind = branch_distance(header, "behind");
            continue;
        }
        let code = line.get(..2).unwrap_or("");
        if code == "??" {
            status.untracked += 1;
        } else if ["DD", "AU", "UD", "UA", "DU", "AA", "UU"].contains(&code) {
            status.conflicted += 1;
        } else {
            let bytes = code.as_bytes();
            if bytes.first().is_some_and(|value| *value != b' ') {
                status.staged += 1;
            }
            if bytes.get(1).is_some_and(|value| *value != b' ') {
                status.modified += 1;
            }
        }
    }
    status.clean = status.staged + status.modified + status.untracked + status.conflicted == 0;
    status
}

/// The distance Git reports in a `## branch...[ahead N, behind M]` header.
fn branch_distance(header: &str, label: &str) -> usize {
    header
        .split(['[', ']', ','])
        .map(str::trim)
        .find_map(|part| part.strip_prefix(label))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0)
}
