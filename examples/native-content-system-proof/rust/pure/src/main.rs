use longhorn_native_content::{
    DetachPolicy, InputRoutingMode, MechanismCapabilities, NativeContentMechanism,
};
use longhorn_native_content_artifact_proof_common::artifact_trace;

fn main() {
    let capabilities = MechanismCapabilities::new(
        NativeContentMechanism::ChildView,
        InputRoutingMode::NativeDirect,
        false,
        DetachPolicy::Reversible,
        false,
        false,
    );
    println!("{}", artifact_trace(capabilities));
}
