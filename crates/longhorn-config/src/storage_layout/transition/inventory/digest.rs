use serde_json::json;

use crate::{Sha256Digest, storage_layout::types::root_kind_id};

use super::super::{
    StorageFileEvidence, StorageTransitionAction, StorageTransitionConflict,
    StorageTransitionConflictKind, StorageTransitionDomain, StorageTransitionRequest,
    StorageTransitionUnknownFile,
};

pub(super) fn evidence_digest(
    domains: &[StorageTransitionDomain],
    source_unknown: &[StorageTransitionUnknownFile],
    target_unknown: &[StorageTransitionUnknownFile],
    conflicts: &[StorageTransitionConflict],
) -> Sha256Digest {
    let domains = domains
        .iter()
        .map(|entry| {
            json!({
                "domain": entry.domain.as_str(),
                "action": action_id(&entry.action),
                "source": evidence_value(entry.source_evidence.as_ref()),
                "target": evidence_value(entry.target_evidence.as_ref()),
            })
        })
        .collect::<Vec<_>>();
    let unknown = |entries: &[StorageTransitionUnknownFile]| {
        entries
            .iter()
            .map(|entry| {
                json!({
                    "root": root_kind_id(entry.root),
                    "path": entry.path.as_os_str().as_encoded_bytes(),
                    "evidence": evidence_value(Some(&entry.evidence)),
                })
            })
            .collect::<Vec<_>>()
    };
    let conflicts = conflicts
        .iter()
        .map(|entry| {
            json!({
                "kind": conflict_id(entry.kind),
                "domain": entry.domain.as_ref().map(|id| id.as_str()),
                "path": entry.path.as_ref().map(|path| path.as_os_str().as_encoded_bytes()),
            })
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&json!({
        "domains": domains,
        "sourceUnknown": unknown(source_unknown),
        "targetUnknown": unknown(target_unknown),
        "conflicts": conflicts,
    }))
    .expect("transition evidence serializes");
    Sha256Digest::from_bytes(&bytes)
}

pub(super) fn confirmation_digest(
    request: &StorageTransitionRequest<'_>,
    evidence: &Sha256Digest,
) -> Sha256Digest {
    let bytes = serde_json::to_vec(&json!({
        "sourceLayout": request.source_layout.digest().as_str(),
        "targetLayout": request.target_layout.digest().as_str(),
        "targetProfile": request.target_selection.profile().id(),
        "targetRoot": request.target_selection.explicit_root().map(|path| path.as_os_str().as_encoded_bytes()),
        "evidence": evidence.as_str(),
    }))
    .expect("transition confirmation serializes");
    Sha256Digest::from_bytes(&bytes)
}

fn evidence_value(evidence: Option<&StorageFileEvidence>) -> serde_json::Value {
    match evidence {
        None => json!(null),
        Some(StorageFileEvidence::Absent) => json!({"state": "absent"}),
        Some(StorageFileEvidence::Present {
            byte_length,
            sha256,
        }) => json!({"state": "present", "bytes": byte_length, "sha256": sha256.as_str()}),
        Some(StorageFileEvidence::Semantic { sha256 }) => {
            json!({"state": "semantic", "sha256": sha256.as_str()})
        }
    }
}

fn action_id(action: &StorageTransitionAction) -> &'static str {
    match action {
        StorageTransitionAction::CopyOrdinary => "copy-ordinary",
        StorageTransitionAction::Absent => "absent",
        StorageTransitionAction::SameAuthority => "same-authority",
        StorageTransitionAction::Identical => "identical",
        StorageTransitionAction::CustomAdapter { .. } => "custom-adapter",
        StorageTransitionAction::Excluded(_) => "excluded",
    }
}

const fn conflict_id(kind: StorageTransitionConflictKind) -> &'static str {
    match kind {
        StorageTransitionConflictKind::OverlappingRoots => "overlapping-roots",
        StorageTransitionConflictKind::TargetOccupied => "target-occupied",
        StorageTransitionConflictKind::UnknownTargetFile => "unknown-target-file",
    }
}
