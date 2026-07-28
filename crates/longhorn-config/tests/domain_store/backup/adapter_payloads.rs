use super::*;

#[test]
fn adapter_payload_paths_are_confined_unique_and_bounded() {
    for invalid in [
        "../escape",
        "/absolute",
        "nested//empty",
        "nested\\windows",
        "space name",
    ] {
        assert!(BackupAdapterRelativePath::new(invalid).is_err());
    }

    let fixture = Fixture::new();
    let domain = OpaqueDomain::new(
        "example.adapter",
        StorageClass::UserConfig,
        "adapter/authority.json",
        &["adapter"],
    );
    let adapter = StaticAdapter::new(vec![
        BackupAdapterPayload::new(
            BackupAdapterRelativePath::new("metadata.json").unwrap(),
            b"metadata".to_vec(),
        ),
        BackupAdapterPayload::new(
            BackupAdapterRelativePath::new("chunks/one.bin").unwrap(),
            b"chunk".to_vec(),
        ),
    ]);
    let mut store = fixture.store();
    store.register(&domain).unwrap();
    let mut catalog = BackupCatalog::new();
    catalog.custom(&domain, &adapter).unwrap();
    let snapshot = super::super::capture(&store, &catalog, &BackupScope::AllRegistered)
        .expect("adapter capture");
    assert_eq!(snapshot.manifest().domains()[0].payloads().len(), 2);
    assert_eq!(
        snapshot.manifest().domains()[0]
            .payloads()
            .iter()
            .map(|payload| payload.path().as_str())
            .collect::<Vec<_>>(),
        [
            "longhorn/adapters/example.adapter/chunks/one.bin",
            "longhorn/adapters/example.adapter/metadata.json",
        ]
    );

    let duplicate_path = BackupAdapterRelativePath::new("duplicate.bin").unwrap();
    let duplicate = StaticAdapter::new(vec![
        BackupAdapterPayload::new(duplicate_path.clone(), vec![1]),
        BackupAdapterPayload::new(duplicate_path, vec![2]),
    ]);
    let mut duplicate_catalog = BackupCatalog::new();
    duplicate_catalog.custom(&domain, &duplicate).unwrap();
    assert!(matches!(
        super::super::capture(&store, &duplicate_catalog, &BackupScope::AllRegistered),
        Err(longhorn_config::BackupCaptureError::InvalidAdapterCapture { .. })
    ));

    let oversized = StaticAdapter::new(vec![BackupAdapterPayload::new(
        BackupAdapterRelativePath::new("oversized.bin").unwrap(),
        vec![0; 5],
    )]);
    let mut oversized_catalog = BackupCatalog::new();
    oversized_catalog.custom(&domain, &oversized).unwrap();
    let limits = longhorn_config::BackupLimits::new(4, 4).unwrap();
    assert!(matches!(
        store.capture_backup(
            &oversized_catalog,
            &BackupScope::AllRegistered,
            super::super::metadata(),
            super::super::options(limits),
        ),
        Err(longhorn_config::BackupCaptureError::DomainTooLarge {
            observed: 5,
            limit: 4,
            ..
        })
    ));
}
