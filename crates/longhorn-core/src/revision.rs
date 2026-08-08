use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

/// Defines one monotonic durable-document revision type.
///
/// Every revision shares the same contract: a transparent `u64`, an
/// `INITIAL` value, and a `checked_next` that fails instead of wrapping.
macro_rules! monotonic_revision {
    (
        $name:ident,
        $overflow:ident,
        $description:literal,
        $initial:literal,
        $overflow_message:literal
    ) => {
        #[doc = $description]
        #[derive(
            Clone,
            Copy,
            Debug,
            Default,
            Deserialize,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            Serialize,
        )]
        #[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
        #[cfg_attr(feature = "bindings", ts(type = "number"))]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            #[doc = $initial]
            pub const INITIAL: Self = Self(0);

            /// Constructs a revision from its serialized value.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Returns the serialized revision value.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }

            /// Returns the next revision or fails instead of wrapping.
            pub const fn checked_next(self) -> Result<Self, $overflow> {
                match self.0.checked_add(1) {
                    Some(value) => Ok(Self(value)),
                    None => Err($overflow),
                }
            }
        }

        /// The revision could not advance without wrapping.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $overflow;

        impl fmt::Display for $overflow {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str($overflow_message)
            }
        }

        impl Error for $overflow {}
    };
}

monotonic_revision!(
    LayoutRevision,
    LayoutRevisionOverflow,
    "Monotonic revision of one durable layout document.",
    "Initial revision for a new layout document.",
    "layout revision cannot advance beyond u64::MAX"
);

monotonic_revision!(
    SurfaceRevision,
    SurfaceRevisionOverflow,
    "Monotonic revision of one durable Surface document.",
    "Initial revision for a new Surface document.",
    "Surface revision cannot advance beyond u64::MAX"
);

monotonic_revision!(
    NativeContentRevision,
    NativeContentRevisionOverflow,
    "Monotonic revision of desired or observed native-content state.",
    "Initial revision for one native-content state channel.",
    "native-content revision cannot advance beyond u64::MAX"
);

monotonic_revision!(
    HistoryRevision,
    HistoryRevisionOverflow,
    "Monotonic structural revision of one history authority.",
    "Initial revision for an empty history authority.",
    "history revision cannot advance beyond u64::MAX"
);

monotonic_revision!(
    OperationRevision,
    OperationRevisionOverflow,
    "Monotonic revision of one asynchronous operation.",
    "Initial revision assigned when an operation is registered.",
    "operation revision cannot advance beyond u64::MAX"
);

monotonic_revision!(
    OperationCatalogueRevision,
    OperationCatalogueRevisionOverflow,
    "Monotonic revision of one operation catalogue.",
    "Initial revision for an empty operation catalogue.",
    "operation catalogue revision cannot advance beyond u64::MAX"
);

monotonic_revision!(
    NotificationLedgerRevision,
    NotificationLedgerRevisionOverflow,
    "Monotonic structural revision of one notification ledger.",
    "Initial revision for an empty notification ledger.",
    "notification ledger revision cannot advance beyond u64::MAX"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_is_monotonic_and_never_wraps() {
        assert_eq!(LayoutRevision::INITIAL.checked_next().unwrap().get(), 1);
        assert_eq!(
            LayoutRevision::new(u64::MAX).checked_next(),
            Err(LayoutRevisionOverflow)
        );
        assert_eq!(SurfaceRevision::INITIAL.checked_next().unwrap().get(), 1);
        assert_eq!(
            SurfaceRevision::new(u64::MAX).checked_next(),
            Err(SurfaceRevisionOverflow)
        );
        assert_eq!(
            NativeContentRevision::INITIAL.checked_next().unwrap().get(),
            1
        );
        assert_eq!(
            NativeContentRevision::new(u64::MAX).checked_next(),
            Err(NativeContentRevisionOverflow)
        );
        assert_eq!(HistoryRevision::INITIAL.checked_next().unwrap().get(), 1);
        assert_eq!(
            HistoryRevision::new(u64::MAX).checked_next(),
            Err(HistoryRevisionOverflow)
        );
        assert_eq!(OperationRevision::INITIAL.checked_next().unwrap().get(), 1);
        assert_eq!(
            OperationRevision::new(u64::MAX).checked_next(),
            Err(OperationRevisionOverflow)
        );
        assert_eq!(
            OperationCatalogueRevision::INITIAL
                .checked_next()
                .unwrap()
                .get(),
            1
        );
        assert_eq!(
            OperationCatalogueRevision::new(u64::MAX).checked_next(),
            Err(OperationCatalogueRevisionOverflow)
        );
        assert_eq!(
            NotificationLedgerRevision::INITIAL
                .checked_next()
                .unwrap()
                .get(),
            1
        );
        assert_eq!(
            NotificationLedgerRevision::new(u64::MAX).checked_next(),
            Err(NotificationLedgerRevisionOverflow)
        );
    }

    #[test]
    fn revision_serializes_as_an_integer() {
        let revision = LayoutRevision::new(42);
        let json = serde_json::to_string(&revision).unwrap();

        assert_eq!(json, "42");
        assert_eq!(
            serde_json::from_str::<LayoutRevision>(&json).unwrap(),
            revision
        );

        let surface_revision = SurfaceRevision::new(73);
        assert_eq!(
            serde_json::from_str::<SurfaceRevision>(
                &serde_json::to_string(&surface_revision).unwrap()
            )
            .unwrap(),
            surface_revision
        );

        let history_revision = HistoryRevision::new(91);
        assert_eq!(
            serde_json::from_str::<HistoryRevision>(
                &serde_json::to_string(&history_revision).unwrap()
            )
            .unwrap(),
            history_revision
        );

        let operation_revision = OperationRevision::new(103);
        assert_eq!(
            serde_json::from_str::<OperationRevision>(
                &serde_json::to_string(&operation_revision).unwrap()
            )
            .unwrap(),
            operation_revision
        );

        let catalogue_revision = OperationCatalogueRevision::new(107);
        assert_eq!(
            serde_json::from_str::<OperationCatalogueRevision>(
                &serde_json::to_string(&catalogue_revision).unwrap()
            )
            .unwrap(),
            catalogue_revision
        );

        let notification_revision = NotificationLedgerRevision::new(109);
        assert_eq!(
            serde_json::from_str::<NotificationLedgerRevision>(
                &serde_json::to_string(&notification_revision).unwrap()
            )
            .unwrap(),
            notification_revision
        );
    }
}
