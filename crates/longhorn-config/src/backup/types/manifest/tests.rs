use serde_json::json;

use super::{BACKUP_FORMAT, BackupManifest};

#[test]
fn strict_manifest_rejects_unknown_format_version_and_fields() {
    let base = json!({
        "format": BACKUP_FORMAT,
        "formatVersion": 1,
        "archiveId": "archive-1",
        "kind": "operational",
        "createdAt": "2026-07-28T12:00:00Z",
        "application": {"id": "com.example.app", "version": "1.0.0"},
        "producer": {"name": "longhorn", "version": "0.1.0"},
        "consistencyGroups": [],
        "domains": [],
        "exclusions": []
    });
    assert!(serde_json::from_value::<BackupManifest>(base.clone()).is_ok());

    let mut future = base.clone();
    future["formatVersion"] = json!(2);
    assert!(serde_json::from_value::<BackupManifest>(future).is_err());

    let mut unknown = base;
    unknown["surprise"] = json!(true);
    assert!(serde_json::from_value::<BackupManifest>(unknown).is_err());

    let mut empty_metadata = json!({
        "format": BACKUP_FORMAT,
        "formatVersion": 1,
        "archiveId": "",
        "kind": "operational",
        "createdAt": "2026-07-28T12:00:00Z",
        "application": {"id": "com.example.app", "version": "1.0.0"},
        "producer": {"name": "longhorn", "version": "0.1.0"},
        "consistencyGroups": [],
        "domains": [],
        "exclusions": []
    });
    assert!(serde_json::from_value::<BackupManifest>(empty_metadata.clone()).is_err());
    empty_metadata["archiveId"] = json!("archive-1");
    empty_metadata["application"]["id"] = json!("");
    assert!(serde_json::from_value::<BackupManifest>(empty_metadata).is_err());
}
