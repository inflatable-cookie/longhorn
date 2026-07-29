//! Optional pure composition between Surface resolution and window hosting.

mod plan;
mod shutdown;

pub use plan::{
    SurfaceWindowBinding, SurfaceWindowCompositionError, SurfaceWindowCompositionErrorCode,
    SurfaceWindowPlan, compose_surface_window_plan,
};
pub use shutdown::{
    SurfaceWindowShutdownError, SurfaceWindowShutdownReceipt, shutdown_surface_window_host,
};
