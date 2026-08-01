use longhorn_native_content_artifact_proof_common::artifact_trace;
use longhorn_tauri_native_content_child_view::CHILD_VIEW_CAPABILITIES;

fn main() {
    println!("{}", artifact_trace(CHILD_VIEW_CAPABILITIES));
}
