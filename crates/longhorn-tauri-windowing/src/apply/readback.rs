use tauri::{AppHandle, Runtime};

use crate::{
    DesktopCoordinateMapper, DesktopObservation, DisplayMetadataProvider, ManagedWindowRegistry,
    TauriObservationError,
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

impl<M> TauriDesktopReadback<M, crate::PlatformDesktopMapper> {
    /// Constructs a production desktop readback for the target platform.
    ///
    /// The coordinate mapper follows from the platform, not from product
    /// policy, so it is not a parameter. See [`PlatformDesktopMapper`].
    ///
    /// [`PlatformDesktopMapper`]: crate::PlatformDesktopMapper
    #[must_use]
    pub fn new(metadata_provider: M) -> Self {
        Self {
            metadata_provider,
            mapper: crate::PlatformDesktopMapper::default(),
        }
    }
}

impl<M, C> TauriDesktopReadback<M, C> {
    /// Constructs a readback through a nominated coordinate mapper.
    ///
    /// [`TauriDesktopReadback::new`] is the ordinary path. Reach for this only
    /// with a reason the platform default does not cover.
    #[must_use]
    pub const fn with_mapper(metadata_provider: M, mapper: C) -> Self {
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
        crate::observe_tauri_desktop_with(
            app,
            &registry.managed_windows(),
            &mut self.metadata_provider,
            &self.mapper,
        )
    }
}
