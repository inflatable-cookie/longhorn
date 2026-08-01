use longhorn_native_content_artifact_proof_common::artifact_trace;
use longhorn_native_content_isolated_window::ISOLATED_WINDOW_CAPABILITIES;

fn main() {
    println!("{}", artifact_trace(ISOLATED_WINDOW_CAPABILITIES));
}
