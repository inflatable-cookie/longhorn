use longhorn_native_content_prototype::{
    AttachGeneration, NativeContentIslandId, NativeContentRevision,
};

/// Private backing-surface adapter failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackingSurfaceError {
    /// A plan named another island.
    ForeignIsland {
        /// Expected island identity.
        expected: NativeContentIslandId,
        /// Supplied island identity.
        supplied: NativeContentIslandId,
    },
    /// A plan or event named another host binding.
    HostBindingMismatch,
    /// A plan contains another native-content mechanism.
    WrongMechanism,
    /// The selected input route is not renderer-forwarded or disabled.
    UnsupportedInputMode,
    /// The selected detach policy differs from the declared fixture policy.
    UnsupportedDetachPolicy,
    /// A plan is older than the most recently admitted desired revision.
    StalePlan {
        /// Current admitted desired revision.
        current: NativeContentRevision,
        /// Supplied plan revision.
        supplied: NativeContentRevision,
    },
    /// A callback or request names an older generation.
    StaleGeneration {
        /// Current generation.
        current: AttachGeneration,
        /// Supplied generation.
        supplied: AttachGeneration,
    },
    /// A callback or request names a future generation.
    FutureGeneration {
        /// Current generation.
        current: AttachGeneration,
        /// Supplied generation.
        supplied: AttachGeneration,
    },
    /// A live attachment prevents a new generation.
    CurrentGenerationAttached(AttachGeneration),
    /// No attachment exists for the requested operation.
    NotAttached,
    /// Shared adapter state was poisoned.
    Poisoned,
    /// Runtime mutation or observation failed.
    Runtime {
        /// Stable operation category.
        operation: &'static str,
        /// Runtime detail retained only as proof evidence.
        detail: String,
    },
    /// The pure receipt builder rejected adapter evidence.
    InvalidReceipt(String),
}

impl BackingSurfaceError {
    pub(crate) const fn failure_code(&self) -> &'static str {
        match self {
            Self::ForeignIsland { .. } => "adapter:foreign-island",
            Self::HostBindingMismatch => "adapter:host-binding",
            Self::WrongMechanism => "adapter:wrong-mechanism",
            Self::UnsupportedInputMode => "adapter:input-mode",
            Self::UnsupportedDetachPolicy => "adapter:detach-policy",
            Self::StalePlan { .. } => "adapter:stale-plan",
            Self::StaleGeneration { .. } => "adapter:stale-generation",
            Self::FutureGeneration { .. } => "adapter:future-generation",
            Self::CurrentGenerationAttached(_) => "adapter:generation-attached",
            Self::NotAttached => "adapter:not-attached",
            Self::Poisoned => "adapter:poisoned",
            Self::Runtime { .. } => "adapter:runtime",
            Self::InvalidReceipt(_) => "adapter:receipt",
        }
    }
}

impl std::fmt::Display for BackingSurfaceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BackingSurfaceError {}
