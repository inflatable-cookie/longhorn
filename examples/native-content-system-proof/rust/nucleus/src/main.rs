use longhorn_native_content_artifact_proof_common::artifact_trace;
use longhorn_native_content::AttachGeneration;
use longhorn_tauri_native_content_child_view::{
    CHILD_VIEW_CAPABILITIES, ChildViewAdapter, ChildViewError, ChildViewNavigationOutcome,
    ChildViewNavigationReceipt, ChildViewRuntime,
};

fn assert_navigation_api<R: ChildViewRuntime>(
    adapter: &ChildViewAdapter<R>,
    generation: AttachGeneration,
    requested_url: tauri::Url,
) -> Result<ChildViewNavigationReceipt, ChildViewError> {
    let _current_url = adapter.current_url(generation)?;
    adapter.navigate(generation, requested_url)
}

fn main() {
    let _outcomes = [
        ChildViewNavigationOutcome::Unchanged,
        ChildViewNavigationOutcome::Submitted,
    ];
    println!("{}", artifact_trace(CHILD_VIEW_CAPABILITIES));
}
