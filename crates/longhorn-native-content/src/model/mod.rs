//! Native-content desired and observed island state.

mod desired;
mod intent;
mod mechanism;
mod observed;

pub use desired::{DesiredState, DesiredUpdate};
pub use intent::{
    AttachmentLifecycle, DesiredPresence, DesiredVisibility, EffectiveFocus, EffectiveVisibility,
    FocusIntent, ObservedGeometry, ObservedReadiness,
};
pub use mechanism::{
    DetachPolicy, InputRoutingMode, MechanismCapabilities, NativeContentMechanism,
};
pub use observed::{ObservationUpdate, ObservedState};
