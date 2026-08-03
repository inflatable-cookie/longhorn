//! External-consumer compile baseline for grouped present/absent restore state.

use longhorn_config::{
    BackupAdapterError, BackupAdapterGroupedApplyKind, BackupAdapterGroupedApplyRequest,
    BackupAdapterGroupedRestore, BackupAdapterGroupedStageRequest,
    BackupAdapterGroupedVerifyRequest, BackupAdapterRestoreStage, BackupAdapterStateEvidence,
    RestoreAdapterGroupPlanEntry, RestoreAdapterGroupReceiptEntry, Sha256Digest,
};
use serde_json::json;

struct PublicGroupedAdapter;

impl BackupAdapterGroupedRestore for PublicGroupedAdapter {
    fn stage(
        &self,
        request: BackupAdapterGroupedStageRequest<'_>,
    ) -> Result<BackupAdapterRestoreStage, BackupAdapterError> {
        let _archive_state = request.inspect().source_state();
        Ok(BackupAdapterRestoreStage::new(
            Vec::new(),
            Vec::new(),
            request.preview().target_evidence().clone(),
            request.preview().current_evidence().clone(),
        ))
    }

    fn apply(
        &self,
        request: BackupAdapterGroupedApplyRequest<'_>,
    ) -> Result<(), BackupAdapterError> {
        let _kind: BackupAdapterGroupedApplyKind = request.kind();
        let _expected: &BackupAdapterStateEvidence = request.expected_evidence();
        Ok(())
    }

    fn verify(
        &self,
        request: BackupAdapterGroupedVerifyRequest<'_>,
    ) -> Result<BackupAdapterStateEvidence, BackupAdapterError> {
        let _kind: BackupAdapterGroupedApplyKind = request.kind();
        Ok(request.expected_evidence().clone())
    }
}

fn project_plan(
    entry: &RestoreAdapterGroupPlanEntry,
) -> (&BackupAdapterStateEvidence, &BackupAdapterStateEvidence) {
    (entry.target_evidence(), entry.rollback_evidence())
}

fn project_receipt(
    entry: &RestoreAdapterGroupReceiptEntry,
) -> (&BackupAdapterStateEvidence, &BackupAdapterStateEvidence) {
    (entry.target_evidence(), entry.rollback_evidence())
}

#[test]
fn grouped_presence_evidence_has_one_explicit_public_shape() {
    let absent = BackupAdapterStateEvidence::Absent;
    let present = BackupAdapterStateEvidence::present(Sha256Digest::from_bytes(b"present"));

    assert_eq!(
        serde_json::to_value(&absent).unwrap(),
        json!({"state": "absent"})
    );
    assert_eq!(
        serde_json::to_value(&present).unwrap(),
        json!({
            "state": "present",
            "sha256": Sha256Digest::from_bytes(b"present").as_str(),
        })
    );
    assert!(absent.is_absent());
    assert_eq!(
        present.sha256(),
        Some(&Sha256Digest::from_bytes(b"present"))
    );

    let _adapter: &dyn BackupAdapterGroupedRestore = &PublicGroupedAdapter;
    let _plan_projection = project_plan;
    let _receipt_projection = project_receipt;
}
