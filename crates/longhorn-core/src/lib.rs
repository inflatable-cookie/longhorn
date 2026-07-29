//! Shared, framework-independent Longhorn primitives.

mod client_geometry;
mod domain_id;
mod geometry;
mod opaque_id;
mod revision;
mod scale;
mod schema_version;
mod window_metrics;

pub use client_geometry::{ClientCssPx, ClientGeometryError, ClientPoint, ClientRect, ClientSize};
pub use domain_id::{DomainId, DomainIdError};
pub use geometry::{
    Coordinate, GeometryError, PhysicalPoint, PhysicalPx, PhysicalRect, PhysicalSize,
    PhysicalSpace, PhysicalVector, Point, Rect, ScreenDip, ScreenPoint, ScreenRect, ScreenSize,
    ScreenSpace, ScreenVector, Size, Vector,
};
pub use opaque_id::{
    DisplayId, DropZoneId, LayoutContainerId, LayoutRequestId, LayoutSchemaId, OpaqueIdError,
    PanelDefinitionId, PanelInstanceId, RegionFamilyId, RegionId, SizingSlotId, SurfaceId,
    SurfaceRequestId, TransferClientId, TransferHostBindingId, TransferRequestId,
    TransferSubjectId, WindowId,
};
pub use revision::{
    LayoutRevision, LayoutRevisionOverflow, SurfaceRevision, SurfaceRevisionOverflow,
};
pub use scale::{RoundingMode, ScaleConversionError, ScaleFactor, ScaleFactorError};
pub use schema_version::{SchemaVersion, SchemaVersionError};
pub use window_metrics::{LiveWindowMetrics, WindowPlacement};
