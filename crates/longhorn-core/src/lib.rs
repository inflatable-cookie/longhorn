//! Shared, framework-independent Longhorn primitives.

mod client_geometry;
mod command_id;
mod domain_id;
mod geometry;
mod opaque_id;
mod revision;
mod scale;
mod schema_version;
mod settings_id;
mod window_metrics;

pub use client_geometry::{ClientCssPx, ClientGeometryError, ClientPoint, ClientRect, ClientSize};
pub use command_id::{
    CommandAvailabilityReasonId, CommandBindingId, CommandCapabilityId, CommandCategoryId,
    CommandContextId, CommandEnumValueId, CommandEvidenceCode, CommandFieldId, CommandId,
    CommandKeymapPresetId, CommandRequestId, CommandRouteId,
};
pub use domain_id::{DomainId, DomainIdError};
pub use geometry::{
    Coordinate, GeometryError, PhysicalPoint, PhysicalPx, PhysicalRect, PhysicalSize,
    PhysicalSpace, PhysicalVector, Point, Rect, ScreenDip, ScreenPoint, ScreenRect, ScreenSize,
    ScreenSpace, ScreenVector, Size, Vector,
};
pub use opaque_id::{
    AuthorityScopeId, BridgeCapabilityId, BridgeCredentialRef, BridgeDiagnosticId, BridgeErrorCode,
    BridgeId, BridgeIdempotencyKey, BridgeJobId, BridgeRequestId, BridgeSessionId, ConfigRequestId,
    DisplayId, DropZoneId, HistoryEntryId, HistoryGroupId, HistoryGroupKeyId, HistoryId,
    HistoryKindId, HistoryPlanId, HostInstanceId, LayoutContainerId, LayoutRequestId,
    LayoutSchemaId, OpaqueIdError, PanelDefinitionId, PanelInstanceId, RegionFamilyId, RegionId,
    SizingSlotId, SurfaceId, SurfaceRequestId, TransferClientId, TransferHostBindingId,
    TransferRequestId, TransferSubjectId, TransportFeatureId, WindowId,
};
pub use revision::{
    HistoryRevision, HistoryRevisionOverflow, LayoutRevision, LayoutRevisionOverflow,
    SurfaceRevision, SurfaceRevisionOverflow,
};
pub use scale::{RoundingMode, ScaleConversionError, ScaleFactor, ScaleFactorError};
pub use schema_version::{SchemaVersion, SchemaVersionError};
pub use settings_id::{
    SettingsActivationTargetId, SettingsAnchorId, SettingsApplyUnitId, SettingsAuthorityToken,
    SettingsCapabilityId, SettingsEntryId, SettingsModuleId, SettingsPageId,
    SettingsPolicySourceId, SettingsRendererId, SettingsRequestId, SettingsScopeId,
    SettingsSectionId,
};
pub use window_metrics::{LiveWindowMetrics, WindowPlacement};
