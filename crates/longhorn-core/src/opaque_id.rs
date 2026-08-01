use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const MAX_OPAQUE_ID_BYTES: usize = 128;

macro_rules! opaque_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
        #[cfg_attr(feature = "bindings", ts(type = "string"))]
        pub struct $name(String);

        impl $name {
            /// Validates and constructs the identifier.
            pub fn new(value: impl Into<String>) -> Result<Self, OpaqueIdError> {
                let value = value.into();
                validate_opaque_id(&value)?;
                Ok(Self(value))
            }

            /// Returns the serialized identifier.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = OpaqueIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(de::Error::custom)
            }
        }
    };
}

opaque_id!(
    DisplayId,
    "Opaque machine-local identity allocated for a known display."
);
opaque_id!(
    WindowId,
    "Product-neutral identity for a logical application window."
);
opaque_id!(
    LayoutSchemaId,
    "Opaque identity for one consumer-registered layout schema."
);
opaque_id!(
    LayoutContainerId,
    "Opaque identity for one Surface-independent layout container."
);
opaque_id!(
    RegionId,
    "Opaque identity for one semantic region in a layout schema."
);
opaque_id!(
    RegionFamilyId,
    "Opaque identity for a consumer-defined family of semantic regions."
);
opaque_id!(
    SizingSlotId,
    "Opaque identity for one consumer-mapped layout sizing control."
);
opaque_id!(
    PanelDefinitionId,
    "Opaque identity for registered product-neutral panel placement policy."
);
opaque_id!(
    PanelInstanceId,
    "Opaque identity for one durable panel instance."
);
opaque_id!(
    LayoutRequestId,
    "Opaque identity for one layout mutation request."
);
opaque_id!(
    ConfigRequestId,
    "Opaque identity for one storage or backup operation request."
);
opaque_id!(
    NativeContentIslandId,
    "Opaque identity for one independently coordinated native-content island."
);
opaque_id!(
    NativeContentKindId,
    "Consumer-owned kind identity for one native-content island."
);
opaque_id!(
    NativeContentRequestId,
    "Correlation identity for one native-content protocol request."
);
opaque_id!(
    NativeContentFailureCode,
    "Stable adapter-owned code for one native-content mechanism failure."
);
opaque_id!(
    VisibilityReasonId,
    "Consumer-owned reason for desired hidden native-content presentation."
);
opaque_id!(
    SurfaceId,
    "Opaque identity for one optional hosted workspace Surface."
);
opaque_id!(
    SurfaceRequestId,
    "Opaque identity for one Surface mutation request."
);
opaque_id!(
    TransferRequestId,
    "Opaque identity for one transfer protocol request."
);
opaque_id!(
    DropZoneId,
    "Opaque identity for one process-local leased transfer target."
);
opaque_id!(
    TransferClientId,
    "Opaque identity for one renderer client participating in transfer."
);
opaque_id!(
    TransferSubjectId,
    "Opaque adapter-supplied identity for one transfer subject."
);
opaque_id!(
    TransferHostBindingId,
    "Opaque adapter-supplied identity for one transfer host binding."
);
opaque_id!(
    BridgeId,
    "Opaque identity for one bridge client installation or process."
);
opaque_id!(
    BridgeSessionId,
    "Opaque identity for one negotiated bridge session."
);
opaque_id!(
    HostInstanceId,
    "Opaque identity for one running authoritative host instance."
);
opaque_id!(
    BridgeCapabilityId,
    "Opaque identity for one domain capability advertised by a bridge host."
);
opaque_id!(
    AuthorityScopeId,
    "Opaque identity for one independently owned bridge authority scope."
);
opaque_id!(
    TransportFeatureId,
    "Opaque identity for one transport-level bridge feature."
);
opaque_id!(
    BridgeDiagnosticId,
    "Opaque stable identity for one bridge diagnostic category."
);
opaque_id!(
    BridgeRequestId,
    "Opaque correlation identity for one bridge request and its replies or events."
);
opaque_id!(
    BridgeIdempotencyKey,
    "Opaque durable replay identity distinct from bridge request correlation."
);
opaque_id!(
    BridgeJobId,
    "Opaque identity for one optional request-correlated bridge job."
);
opaque_id!(
    BridgeErrorCode,
    "Opaque stable code for one domain or bridge protocol failure."
);
opaque_id!(
    BridgeCredentialRef,
    "Opaque reference to consumer-owned credential material."
);
opaque_id!(
    HistoryId,
    "Opaque identity for one independently owned history authority."
);
opaque_id!(
    HistoryEntryId,
    "Stable opaque identity for one retained history entry."
);
opaque_id!(
    HistoryKindId,
    "Consumer-owned kind identity for history metadata and presentation."
);
opaque_id!(
    HistoryGroupId,
    "Explicit identity for one committed history group."
);
opaque_id!(
    HistoryGroupKeyId,
    "Consumer-owned key identity for one timed history grouping policy."
);
opaque_id!(
    HistoryPlanId,
    "Opaque identity for one revision-bound history navigation plan."
);
opaque_id!(
    OperationAuthorityId,
    "Opaque identity for one independently owned operation catalogue."
);
opaque_id!(
    OperationId,
    "Stable opaque identity for one retained asynchronous operation."
);
opaque_id!(
    OperationRequestId,
    "Correlation identity for one operation protocol request."
);
opaque_id!(
    OperationKindId,
    "Consumer-owned kind identity for operation admission and presentation."
);
opaque_id!(
    OperationScopeId,
    "Optional consumer-owned scope identity for an operation."
);
opaque_id!(
    OperationPhaseId,
    "Consumer-owned phase identity for later bounded progress projection."
);
opaque_id!(
    NotificationAuthorityId,
    "Opaque identity for one independently owned notification ledger."
);
opaque_id!(
    NotificationId,
    "Stable opaque identity for one retained notification."
);
opaque_id!(
    NotificationRequestId,
    "Correlation identity for one notification protocol request."
);
opaque_id!(
    NotificationSourceId,
    "Consumer-owned identity for the source of one notification."
);
opaque_id!(
    NotificationReplacementKey,
    "Consumer-owned key for explicit notification replacement."
);
opaque_id!(
    NotificationActionReferenceId,
    "Bounded semantic reference to a consumer-owned notification action."
);
opaque_id!(
    NotificationCauseId,
    "Opaque consumer-owned reference to a notification cause."
);
opaque_id!(
    NotificationProducerToken,
    "Durable producer token for idempotent notification publication."
);
pub(crate) fn validate_opaque_id(value: &str) -> Result<(), OpaqueIdError> {
    if value.is_empty() {
        return Err(OpaqueIdError::Empty);
    }
    if value.len() > MAX_OPAQUE_ID_BYTES {
        return Err(OpaqueIdError::TooLong {
            maximum: MAX_OPAQUE_ID_BYTES,
            actual: value.len(),
        });
    }

    if let Some((index, _)) = value.char_indices().find(|(_, character)| {
        !(character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '_' | ':' | '-'))
    }) {
        return Err(OpaqueIdError::InvalidCharacter { index });
    }

    Ok(())
}

/// Validation failure for an opaque Longhorn identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpaqueIdError {
    /// The identifier was empty.
    Empty,
    /// The identifier exceeded the bounded serialized length.
    TooLong {
        /// Maximum accepted byte length.
        maximum: usize,
        /// Supplied byte length.
        actual: usize,
    },
    /// A character fell outside the stable lowercase ASCII grammar.
    InvalidCharacter {
        /// Byte index of the invalid character.
        index: usize,
    },
}

impl fmt::Display for OpaqueIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("opaque id cannot be empty"),
            Self::TooLong { maximum, actual } => {
                write!(
                    formatter,
                    "opaque id is {actual} bytes; maximum is {maximum}"
                )
            }
            Self::InvalidCharacter { index } => {
                write!(
                    formatter,
                    "opaque id has an invalid character at byte {index}"
                )
            }
        }
    }
}

impl Error for OpaqueIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_strict_bounded_strings() {
        assert_eq!(DisplayId::new(""), Err(OpaqueIdError::Empty));
        assert_eq!(
            WindowId::new("Main"),
            Err(OpaqueIdError::InvalidCharacter { index: 0 })
        );
        assert!(DisplayId::new("0198f97e-8d2a-7e31-a302-6f23098ccb9d").is_ok());
        assert!(WindowId::new("editor:secondary_2").is_ok());
        assert!(OperationAuthorityId::new("authority:desktop").is_ok());
        assert!(OperationId::new("operation:scan-42").is_ok());
        assert!(OperationKindId::new("soundcheck.plugin-scan").is_ok());
        assert!(OperationScopeId::new("library:default").is_ok());
        assert!(OperationPhaseId::new("scan:introspecting").is_ok());
        assert!(NotificationAuthorityId::new("notifications:desktop").is_ok());
        assert!(NotificationId::new("notification:render-42").is_ok());
        assert!(NotificationSourceId::new("loophole.render").is_ok());
        assert!(NotificationReplacementKey::new("render:42").is_ok());
        assert!(NotificationActionReferenceId::new("render:open-report").is_ok());
        assert!(NotificationCauseId::new("operation:render-42").is_ok());
        assert!(NotificationProducerToken::new("operation:render-42:failed").is_ok());
        assert_eq!(LayoutSchemaId::new(""), Err(OpaqueIdError::Empty));
        assert_eq!(
            RegionId::new("Main"),
            Err(OpaqueIdError::InvalidCharacter { index: 0 })
        );
        assert_eq!(
            PanelDefinitionId::new("panel tool"),
            Err(OpaqueIdError::InvalidCharacter { index: 5 })
        );
        assert_eq!(
            PanelInstanceId::new("x".repeat(MAX_OPAQUE_ID_BYTES + 1)),
            Err(OpaqueIdError::TooLong {
                maximum: MAX_OPAQUE_ID_BYTES,
                actual: MAX_OPAQUE_ID_BYTES + 1,
            })
        );
        assert!(SurfaceId::new("surface:workspace_2").is_ok());
        assert_eq!(
            SurfaceRequestId::new("Request"),
            Err(OpaqueIdError::InvalidCharacter { index: 0 })
        );
        assert!(DropZoneId::new("drop:workspace_tools").is_ok());
        assert!(TransferClientId::new("client:main").is_ok());
        assert!(TransferSubjectId::new("panel:inspector").is_ok());
        assert!(TransferHostBindingId::new("host:main").is_ok());
        assert!(BridgeId::new("bridge:renderer").is_ok());
        assert!(BridgeSessionId::new("session:0198f97e").is_ok());
        assert!(HostInstanceId::new("host:embedded").is_ok());
        assert!(BridgeCapabilityId::new("query").is_ok());
        assert!(AuthorityScopeId::new("scope:workspace").is_ok());
        assert!(TransportFeatureId::new("request_reply").is_ok());
        assert!(BridgeDiagnosticId::new("authority:degraded").is_ok());
        assert!(BridgeRequestId::new("request:0198f97e").is_ok());
        assert!(BridgeIdempotencyKey::new("idempotency:0198f97e").is_ok());
        assert!(BridgeJobId::new("job:0198f97e").is_ok());
        assert!(BridgeErrorCode::new("workspace:stale").is_ok());
        assert!(HistoryId::new("history:project").is_ok());
        assert!(HistoryEntryId::new("entry:0198f97e").is_ok());
        assert!(HistoryKindId::new("track:rename").is_ok());
        assert!(HistoryGroupId::new("gesture:0198f97e").is_ok());
        assert!(HistoryGroupKeyId::new("gesture:clip_move").is_ok());
        assert!(HistoryPlanId::new("plan:0198f97e").is_ok());
    }

    #[test]
    fn serde_round_trips_without_type_substitution() {
        let display = DisplayId::new("display:0198f97e").unwrap();
        let window = WindowId::new("window:primary").unwrap();

        assert_eq!(
            serde_json::to_string(&display).unwrap(),
            "\"display:0198f97e\""
        );
        assert_eq!(
            serde_json::from_str::<DisplayId>("\"display:0198f97e\"").unwrap(),
            display
        );
        assert_eq!(
            serde_json::from_str::<WindowId>("\"window:primary\"").unwrap(),
            window
        );

        let container = LayoutContainerId::new("layout:primary").unwrap();
        assert_eq!(
            serde_json::from_str::<LayoutContainerId>(&serde_json::to_string(&container).unwrap())
                .unwrap(),
            container
        );

        let surface = SurfaceId::new("surface:primary").unwrap();
        assert_eq!(
            serde_json::from_str::<SurfaceId>(&serde_json::to_string(&surface).unwrap()).unwrap(),
            surface
        );
    }
}
