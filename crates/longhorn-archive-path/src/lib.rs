//! The archive path rule (contracts 018 and 027).
//!
//! An archive entry name, and a link target, may not escape the directory it
//! unpacks into. This is the one rule both bounded extractions use:
//! `longhorn-update-install` (contract 018) and `longhorn-tool-provision`
//! (contract 027). It lives here so the two cannot drift.
//!
//! The rule is a component check on the path as written. Callers that also
//! need the filesystem's view (a link an earlier entry planted) add their own
//! check on top; the rule does not touch the disk.

use std::{
    fmt,
    path::{Component, Path, PathBuf},
};

/// Why [`bounded`] refused a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathRefusal {
    /// The path has an absolute, drive-prefix or `..` component.
    Escapes {
        /// The path as the archive wrote it.
        path: PathBuf,
    },
    /// The path has no normal components to write to.
    Empty,
}

impl fmt::Display for PathRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Escapes { path } => write!(
                formatter,
                "archive entry escapes the destination: {}",
                path.display()
            ),
            Self::Empty => formatter.write_str("archive entry has an empty path"),
        }
    }
}

impl std::error::Error for PathRefusal {}

/// The normal components of an archive path, as a relative path.
///
/// `.` components are dropped. A root, a drive prefix or a `..` component
/// refuses with [`PathRefusal::Escapes`]; a path with nothing left refuses
/// with [`PathRefusal::Empty`]. Backslashes are ordinary characters here: a
/// caller that refuses them does so on top of this rule.
pub fn bounded(path: &Path) -> Result<PathBuf, PathRefusal> {
    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PathRefusal::Escapes {
                    path: path.to_path_buf(),
                });
            }
        }
    }
    if safe.as_os_str().is_empty() {
        return Err(PathRefusal::Empty);
    }
    Ok(safe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_paths_pass_and_curdir_is_dropped() {
        assert_eq!(
            bounded(Path::new("./bin/gh")).unwrap(),
            PathBuf::from("bin/gh")
        );
        assert_eq!(
            bounded(Path::new("gh_2/bin/gh")).unwrap(),
            PathBuf::from("gh_2/bin/gh")
        );
    }

    #[test]
    fn escapes_and_empty_paths_refuse_with_the_update_wording() {
        for escaping in ["../x", "/abs", "a/../../x"] {
            let refusal = bounded(Path::new(escaping)).unwrap_err();
            assert!(matches!(refusal, PathRefusal::Escapes { .. }), "{escaping}");
        }
        let empty = bounded(Path::new(".")).unwrap_err();
        assert_eq!(empty, PathRefusal::Empty);
        assert_eq!(empty.to_string(), "archive entry has an empty path");
        assert_eq!(
            bounded(Path::new("../x")).unwrap_err().to_string(),
            "archive entry escapes the destination: ../x"
        );
    }
}
