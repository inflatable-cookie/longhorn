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
///
/// Only the trailing bracketed summary is read. The branch name itself can
/// start with a label (for example `ahead-topic`), so matching against the
/// whole header lets the branch mask the real field and report zero.
fn branch_distance(header: &str, label: &str) -> usize {
    let Some(summary) = trailing_summary(header) else {
        return 0;
    };
    summary
        .split(',')
        .map(str::trim)
        .find_map(|item| {
            let value = item.strip_prefix(label)?;
            let value = value.strip_prefix(char::is_whitespace)?;
            let value = value.trim();
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            value.parse().ok()
        })
        .unwrap_or(0)
}

/// The trailing `[ahead N, behind M]` summary of a status branch header.
///
/// Branch names cannot contain `[`, so the last bracket opens the summary
/// even when the branch name holds `]` or `,`.
fn trailing_summary(header: &str) -> Option<&str> {
    let body = header.trim_end().strip_suffix(']')?;
    let open = body.rfind('[')?;
    Some(&body[open + 1..])
}

#[cfg(test)]
mod tests {
    use super::read_status;

    fn header_status(header: &str) -> (usize, usize) {
        let status = read_status(&format!("## {header}\n"));
        (status.ahead, status.behind)
    }

    #[test]
    fn reads_ahead_and_behind_from_the_trailing_summary() {
        assert_eq!(
            header_status("main...origin/main [ahead 1, behind 2]"),
            (1, 2)
        );
        assert_eq!(header_status("main...origin/main [ahead 1]"), (1, 0));
        assert_eq!(header_status("main...origin/main [behind 3]"), (0, 3));
        assert_eq!(header_status("main...origin/main"), (0, 0));
        assert_eq!(header_status("main"), (0, 0));
    }

    #[test]
    fn branch_names_starting_with_ahead_never_mask_the_summary() {
        assert_eq!(
            header_status("ahead-topic...origin/ahead-topic [ahead 2, behind 1]"),
            (2, 1)
        );
        assert_eq!(header_status("ahead...origin/ahead [behind 4]"), (0, 4));
        assert_eq!(header_status("ahead-topic...origin/ahead-topic"), (0, 0));
        assert_eq!(header_status("ahead"), (0, 0));
    }

    #[test]
    fn branch_names_starting_with_behind_never_mask_the_summary() {
        assert_eq!(
            header_status("behind-topic...origin/behind-topic [ahead 1, behind 3]"),
            (1, 3)
        );
        assert_eq!(header_status("behind...origin/behind [ahead 2]"), (2, 0));
        assert_eq!(header_status("behind-topic...origin/behind-topic"), (0, 0));
        assert_eq!(header_status("behind"), (0, 0));
    }

    #[test]
    fn branch_names_starting_with_gone_leave_distances_at_zero() {
        assert_eq!(
            header_status("gone-fishing...origin/gone-fishing [gone]"),
            (0, 0)
        );
        assert_eq!(
            header_status("gone-fishing...origin/gone-fishing [ahead 1]"),
            (1, 0)
        );
        assert_eq!(header_status("gone-fishing...origin/gone-fishing"), (0, 0));
    }

    #[test]
    fn detached_and_unborn_headers_have_no_distance() {
        assert_eq!(header_status("HEAD (no branch)"), (0, 0));
        assert_eq!(header_status("No commits yet on main"), (0, 0));
    }
}
