use longhorn_core::DomainId;
use serde_json::json;

use crate::{Sha256Digest, StorageFileEvidence};

pub(crate) fn receipt_digest(
    transition_id: &str,
    target_layout: &Sha256Digest,
    copied: &[DomainId],
    custom: &[DomainId],
    retained: &[std::path::PathBuf],
    retained_evidence: &[StorageFileEvidence],
) -> Sha256Digest {
    let bytes = serde_json::to_vec(&json!({
        "transitionId": transition_id,
        "targetLayout": target_layout.as_str(),
        "copied": copied.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "custom": custom.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "retained": retained.iter().zip(retained_evidence).map(|(path, evidence)| json!({
            "path": path.as_os_str().as_encoded_bytes(),
            "evidence": evidence_json(evidence),
        })).collect::<Vec<_>>(),
    }))
    .expect("transition receipt serializes");
    Sha256Digest::from_bytes(&bytes)
}

fn evidence_json(evidence: &StorageFileEvidence) -> serde_json::Value {
    match evidence {
        StorageFileEvidence::Absent => json!({"kind": "absent"}),
        StorageFileEvidence::Present {
            byte_length,
            sha256,
        } => json!({
            "kind": "present",
            "byteLength": byte_length,
            "sha256": sha256.as_str(),
        }),
        StorageFileEvidence::Semantic { sha256 } => {
            json!({"kind": "semantic", "sha256": sha256.as_str()})
        }
    }
}
