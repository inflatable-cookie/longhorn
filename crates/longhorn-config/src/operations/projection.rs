use std::path::Path;

mod backup;
mod restore;
mod storage;

use super::ConfigOperationProjectionError;

pub(super) fn exact_path(path: &Path) -> Result<String, ConfigOperationProjectionError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(ConfigOperationProjectionError)
}

#[cfg(test)]
mod tests {
    use crate::{
        PlatformDirectoryFacts, StorageIdentity, StorageLayoutProjection, StorageLayoutRequest,
        TargetPlatform, resolve_storage_layout,
    };

    #[test]
    fn resolved_layout_projects_exact_identity_roots_and_provenance() {
        let facts = PlatformDirectoryFacts::complete(
            TargetPlatform::MacOs,
            "/native/config",
            "/native/data",
            "/native/state",
            "/native/cache",
            "/native/log",
            "/native/runtime",
        );
        let layout = resolve_storage_layout(&StorageLayoutRequest::new(
            StorageIdentity::new("audio.infiniteloop.soundcheck").unwrap(),
            facts,
        ))
        .unwrap();
        let projection = StorageLayoutProjection::try_from(&layout.diagnostic()).unwrap();
        assert_eq!(
            projection.canonical_application_id,
            "audio.infiniteloop.soundcheck"
        );
        assert_eq!(
            projection.roots[0].path,
            "/native/config/audio.infiniteloop.soundcheck/config"
        );
        assert_eq!(projection.roots[0].provenance, "platform:config");
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_fail_instead_of_becoming_lossy_evidence() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

        let path = PathBuf::from(OsString::from_vec(vec![b'/', 0xff]));
        assert_eq!(
            super::exact_path(&path),
            Err(super::ConfigOperationProjectionError)
        );
    }
}
