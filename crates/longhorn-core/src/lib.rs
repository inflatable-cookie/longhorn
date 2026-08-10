//! Shared, framework-independent Longhorn primitives.

mod client_geometry;
mod command_id;
mod diagnostics;
mod domain_id;
mod geometry;
mod host_services;
mod opaque_id;
mod revision;
mod scale;
mod schema_version;
mod settings_id;
mod store_compatibility;
mod window_metrics;

pub use client_geometry::{
    ClientGeometryError, ClientLogicalPx, ClientPoint, ClientRect, ClientSize,
};
pub use command_id::{
    CommandAvailabilityReasonId, CommandBindingId, CommandCapabilityId, CommandCategoryId,
    CommandContextId, CommandEnumValueId, CommandEvidenceCode, CommandFieldId, CommandId,
    CommandKeymapPresetId, CommandRequestId, CommandRouteId,
};
pub use diagnostics::{
    BestEffortDiagnostics, install_best_effort_diagnostics, report_best_effort_failure,
};
pub use domain_id::{DomainId, DomainIdError};
pub use geometry::{
    Coordinate, GeometryError, PhysicalPoint, PhysicalPx, PhysicalRect, PhysicalSize,
    PhysicalSpace, PhysicalVector, Point, Rect, ScreenDip, ScreenPoint, ScreenRect, ScreenSize,
    ScreenSpace, ScreenVector, Size, Vector,
};
pub use host_services::{HostServices, PlainHostServices};
pub use opaque_id::MAX_OPAQUE_ID_BYTES;
pub use opaque_id::{
    AuthorityScopeId, BridgeCapabilityId, BridgeCredentialRef, BridgeDiagnosticId, BridgeErrorCode,
    BridgeId, BridgeIdempotencyKey, BridgeJobId, BridgeRequestId, BridgeSessionId, ConfigRequestId,
    DisplayId, DropZoneId, HistoryEntryId, HistoryGroupId, HistoryGroupKeyId, HistoryId,
    HistoryKindId, HistoryPlanId, HostInstanceId, LayoutRequestId, LayoutSchemaId,
    NativeContentFailureCode, NativeContentIslandId, NativeContentKindId, NativeContentRequestId,
    NotificationActionReferenceId, NotificationAuthorityId, NotificationCauseId, NotificationId,
    NotificationProducerToken, NotificationReplacementKey, NotificationRequestId,
    NotificationSourceId, OpaqueIdError, OperationAuthorityId, OperationId, OperationKindId,
    OperationPhaseId, OperationRequestId, OperationScopeId, PanelDefinitionId, PanelInstanceId,
    RegionFamilyId, RegionId, SizingSlotId, SurfaceId, SurfaceRequestId, TransferClientId,
    TransferHostBindingId, TransferRequestId, TransferSubjectId, TransportFeatureId,
    VisibilityReasonId, WindowId,
};
pub use revision::{
    HistoryRevision, HistoryRevisionOverflow, LayoutRevision, LayoutRevisionOverflow,
    NativeContentRevision, NativeContentRevisionOverflow, NotificationLedgerRevision,
    NotificationLedgerRevisionOverflow, OperationCatalogueRevision,
    OperationCatalogueRevisionOverflow, OperationRevision, OperationRevisionOverflow,
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
pub use store_compatibility::{CompatibilityStore, FutureSchemaRefusal, FutureSchemaRefused};
pub use window_metrics::{LiveWindowMetrics, WindowPlacement};
