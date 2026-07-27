use std::path::PathBuf;

use longhorn_core::{DomainId, SchemaVersion};

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

#[test]
fn roots_must_be_absolute() {
    let result = StorageRoots::new("config", "/data", "/cache", "/tmp", "/logs");

    assert_eq!(
        result,
        Err(StorageRootError {
            kind: RootKind::Config,
            path: PathBuf::from("config")
        })
    );
}

#[test]
fn non_file_authorities_stay_typed() {
    let roots = StorageRoots::new("/config", "/data", "/cache", "/tmp", "/logs").unwrap();

    assert_eq!(
        roots.resolve(&descriptor(StorageClass::Defaults)),
        DomainLocation::DefaultsOnly
    );
    assert_eq!(
        roots.resolve(&descriptor(StorageClass::Secret)),
        DomainLocation::SecureStoreRequired
    );
    assert!(matches!(
        roots.resolve(&descriptor(StorageClass::ProjectShared)),
        DomainLocation::RootRequired {
            root: RootKind::Project,
            ..
        }
    ));
}
