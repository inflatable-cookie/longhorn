use tauri::{AppHandle, Runtime};

use crate::{
    DesktopCoordinateMapper, DesktopObservation, DisplayMetadataProvider, ManagedWindowRegistry,
    TauriObservationError, observe_tauri_desktop,
};

/// Fresh complete desktop observation after native mutation attempts.
pub trait ManagedDesktopReadback<R: Runtime>: Send {
    /// Reads displays and every explicitly managed window.
    fn readback(
        &mut self,
        app: &AppHandle<R>,
        registry: &ManagedWindowRegistry<R>,
    ) -> Result<DesktopObservation, TauriObservationError>;
}

/// Production readback using the Card 017 probe and coordinate mapper.
pub struct TauriDesktopReadback<M, C> {
    metadata_provider: M,
    mapper: C,
}

impl<M, C> TauriDesktopReadback<M, C> {
    /// Constructs a production desktop readback.
    #[must_use]
    pub const fn new(metadata_provider: M, mapper: C) -> Self {
        Self {
            metadata_provider,
            mapper,
        }
    }
}

impl<R, M, C> ManagedDesktopReadback<R> for TauriDesktopReadback<M, C>
where
    R: Runtime,
    M: DisplayMetadataProvider + Send,
    C: DesktopCoordinateMapper + Send,
{
    fn readback(
        &mut self,
        app: &AppHandle<R>,
        registry: &ManagedWindowRegistry<R>,
    ) -> Result<DesktopObservation, TauriObservationError> {
        observe_tauri_desktop(
            app,
            &registry.managed_windows(),
            &mut self.metadata_provider,
            &self.mapper,
        )
    }
}
