use std::fs;

use longhorn_core::{DomainId, SchemaVersion};
use tempfile::TempDir;

use crate::{
    AccessMode, DomainDescriptor, DomainFilePath, DomainLocation, RootKind, StorageClass,
    StorageRoots,
};

use super::*;

fn fixture() -> (TempDir, ResolvedFile) {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("config");
    fs::create_dir_all(&root).unwrap();
    let roots = StorageRoots::new(
        &root,
        temp.path().join("data"),
        temp.path().join("cache"),
        temp.path().join("runtime"),
        temp.path().join("log"),
    )
    .unwrap();
    let descriptor = DomainDescriptor::new(
        DomainId::new("example.preferences").unwrap(),
        SchemaVersion::new(1).unwrap(),
        StorageClass::UserConfig,
        Some(DomainFilePath::new("example/preferences.json").unwrap()),
    )
    .unwrap();
    let DomainLocation::File(file) = roots.resolve(&descriptor) else {
        panic!("expected file");
    };
    assert_eq!(file.root_kind(), RootKind::Config);
    assert_eq!(file.access(), AccessMode::ReadWrite);
    (temp, file)
}

#[test]
fn every_failure_before_rename_preserves_target_and_cleans_temporary_files() {
    for stage in [
        PublicationStage::OpenRoot,
        PublicationStage::CreateParent,
        PublicationStage::OpenParent,
        PublicationStage::CreateTemporary,
        PublicationStage::WriteTemporary,
        PublicationStage::SyncTemporary,
        PublicationStage::Rename,
    ] {
        let (_temp, target) = fixture();
        fs::create_dir_all(target.full_path().parent().unwrap()).unwrap();
        fs::write(target.full_path(), b"old").unwrap();

        let error =
            publish_inner(&target, b"new", DurabilityRequirement::Atomic, Some(stage)).unwrap_err();

        assert_eq!(error.stage, stage);
        assert!(!error.published);
        assert_eq!(fs::read(target.full_path()).unwrap(), b"old");
        let entries = fs::read_dir(target.full_path().parent().unwrap())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(entries.len(), 1);
    }
}

#[test]
fn durable_requirement_reports_post_publication_sync_failure() {
    let (_temp, target) = fixture();

    let error = publish_inner(
        &target,
        b"new",
        DurabilityRequirement::Durable,
        Some(PublicationStage::SyncDirectory),
    )
    .unwrap_err();

    assert_eq!(error.stage, PublicationStage::SyncDirectory);
    assert!(error.published);
    assert_eq!(fs::read(target.full_path()).unwrap(), b"new");
}
