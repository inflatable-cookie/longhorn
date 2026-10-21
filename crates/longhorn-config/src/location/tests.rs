use std::path::Path;

use longhorn_core::{DomainId, SchemaVersion};
use longhorn_storage_layout::{RootKind, StorageRoots};

use super::*;

fn descriptor(class: StorageClass) -> DomainDescriptor {
    let path = if matches!(class, StorageClass::Defaults | StorageClass::Secret) {
        None
    } else {
        Some(DomainFilePath::new("example/settings.json").unwrap())
    };

    DomainDescriptor::new(
        DomainId::new("example.settings").unwrap(),
        SchemaVersion::new(1).unwrap(),
        class,
        path,
    )
    .unwrap()
}

fn roots() -> StorageRoots {
    StorageRoots::new(
        "/config", "/data", "/state", "/cache", "/tmp", "/logs", "/backups",
    )
    .unwrap()
}

#[test]
fn non_file_authorities_stay_typed() {
    assert_eq!(
        roots().resolve(&descriptor(StorageClass::Defaults)),
        DomainLocation::DefaultsOnly
    );
    assert_eq!(
        roots().resolve(&descriptor(StorageClass::Secret)),
        DomainLocation::SecureStoreRequired
    );
}

#[test]
fn optional_roots_resolve_or_report_required() {
    assert!(matches!(
        roots().resolve(&descriptor(StorageClass::ProjectShared)),
        DomainLocation::RootRequired {
            root: RootKind::Project,
            ..
        }
    ));

    let project_roots = roots().with_project("/project").unwrap();
    assert!(matches!(
        project_roots.resolve(&descriptor(StorageClass::ProjectShared)),
        DomainLocation::File(_)
    ));
}

#[test]
fn file_backed_classes_confine_to_their_root() {
    let location = roots().resolve(&descriptor(StorageClass::UserConfig));
    let DomainLocation::File(file) = location else {
        panic!("user config must resolve to a file");
    };
    assert_eq!(file.root_kind(), RootKind::Config);
    assert_eq!(file.root(), Path::new("/config"));
    assert_eq!(file.full_path(), Path::new("/config/example/settings.json"));
}
