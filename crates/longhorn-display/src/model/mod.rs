//! Display inventory model types.

mod displays;
mod evidence;
mod facts;
mod ids;
mod registry;

pub use displays::{KnownDisplay, ObservedDisplay};
pub use evidence::DisplayEvidence;
pub use facts::{DisplayBuiltinStatus, DisplayFacts};
pub use ids::{
    AdapterDisplayKey, DisplayLabel, ObservationId, StrongDisplayKey, WeakDisplayKey,
};
pub use registry::{KnownDisplayRegistry, RegistryError};
