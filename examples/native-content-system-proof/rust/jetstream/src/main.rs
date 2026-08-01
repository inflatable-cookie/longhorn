use longhorn_native_content_artifact_proof_common::artifact_trace;
use longhorn_native_content_backing_surface::BACKING_SURFACE_CAPABILITIES;

fn main() {
    println!("{}", artifact_trace(BACKING_SURFACE_CAPABILITIES));
}
