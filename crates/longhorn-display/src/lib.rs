//! Pure display inventory, correlation, and arrangement primitives.

mod arrangement;
mod model;
mod outcome;
mod reconcile;
mod text;

pub use arrangement::{ArrangementSignature, ArrangementSignatureError};
pub use model::{
    AdapterDisplayKey, DisplayBuiltinStatus, DisplayEvidence, DisplayFacts, DisplayLabel,
    KnownDisplay, KnownDisplayRegistry, ObservationId, ObservedDisplay, RegistryError,
    StrongDisplayKey, WeakDisplayKey,
};
pub use outcome::{
    AssociationEvidence, AssociationKind, CorrelationAmbiguity, CorrelationConfidence,
    CorrelationMatch, DisplayAvailability, DisplayIdAllocator, DisplayInventory, InventoryDisplay,
    ReconcileError, Reconciliation, UnresolvedObservation, UnresolvedReason,
};
pub use reconcile::reconcile_displays;
pub use text::DisplayTextError;
