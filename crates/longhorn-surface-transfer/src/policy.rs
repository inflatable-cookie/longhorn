use std::collections::BTreeSet;
use std::{error::Error, fmt};

use longhorn_core::{DisplayId, ScreenPoint, ScreenRect, WindowId, WindowPlacement};
use longhorn_surfaces::EmptyWindowPolicy;

/// One consumer-approved logical window on an otherwise empty display.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmptyDisplayProvisionTarget {
    display_id: DisplayId,
    display_bounds: ScreenRect,
    window_id: WindowId,
    placement: WindowPlacement,
    insertion_index: Option<u32>,
}

impl EmptyDisplayProvisionTarget {
    /// Constructs one target from consumer-resolved display and placement policy.
    #[must_use]
    pub const fn new(
        display_id: DisplayId,
        display_bounds: ScreenRect,
        window_id: WindowId,
        placement: WindowPlacement,
        insertion_index: Option<u32>,
    ) -> Self {
        Self {
            display_id,
            display_bounds,
            window_id,
            placement,
            insertion_index,
        }
    }

    /// Returns the stable display identity.
    #[must_use]
    pub const fn display_id(&self) -> &DisplayId {
        &self.display_id
    }

    /// Returns fresh display bounds used for hit-testing.
    #[must_use]
    pub const fn display_bounds(&self) -> ScreenRect {
        self.display_bounds
    }

    /// Returns the predeclared logical Surface host.
    #[must_use]
    pub const fn window_id(&self) -> &WindowId {
        &self.window_id
    }

    /// Returns consumer-resolved target placement.
    #[must_use]
    pub const fn placement(&self) -> WindowPlacement {
        self.placement
    }

    /// Returns optional target membership insertion policy.
    #[must_use]
    pub const fn insertion_index(&self) -> Option<u32> {
        self.insertion_index
    }
}

/// Explicit empty-display provisioning posture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmptyDisplayProvisionPolicy {
    /// Never create a native window from an empty-display drop.
    Disabled,
    /// Admit only the supplied fresh display targets.
    Enabled(Vec<EmptyDisplayProvisionTarget>),
}

/// Complete consumer policy rechecked by one terminal Surface attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceTransferPolicy {
    allowed_target_windows: BTreeSet<WindowId>,
    empty_window_policy: EmptyWindowPolicy,
    empty_display_policy: EmptyDisplayProvisionPolicy,
}

impl SurfaceTransferPolicy {
    /// Constructs and validates one complete policy snapshot.
    pub fn new(
        allowed_target_windows: impl IntoIterator<Item = WindowId>,
        empty_window_policy: EmptyWindowPolicy,
        empty_display_policy: EmptyDisplayProvisionPolicy,
    ) -> Result<Self, SurfaceTransferPolicyError> {
        let allowed_target_windows = allowed_target_windows.into_iter().collect();
        validate_empty_targets(&empty_display_policy)?;
        Ok(Self {
            allowed_target_windows,
            empty_window_policy,
            empty_display_policy,
        })
    }

    /// Constructs policy with provisioning disabled by default.
    #[must_use]
    pub fn provisioning_disabled(
        allowed_target_windows: impl IntoIterator<Item = WindowId>,
        empty_window_policy: EmptyWindowPolicy,
    ) -> Self {
        Self {
            allowed_target_windows: allowed_target_windows.into_iter().collect(),
            empty_window_policy,
            empty_display_policy: EmptyDisplayProvisionPolicy::Disabled,
        }
    }

    pub(crate) fn allows_window(&self, window_id: &WindowId) -> bool {
        self.allowed_target_windows.contains(window_id)
    }

    pub(crate) const fn empty_window_policy(&self) -> EmptyWindowPolicy {
        self.empty_window_policy
    }

    pub(crate) fn empty_target(
        &self,
        point: ScreenPoint,
    ) -> Result<&EmptyDisplayProvisionTarget, EmptyTargetResolution> {
        let EmptyDisplayProvisionPolicy::Enabled(targets) = &self.empty_display_policy else {
            return Err(EmptyTargetResolution::Disabled);
        };
        let matches = targets
            .iter()
            .filter(|target| target.display_bounds.contains_point(&point))
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [target] => Ok(target),
            [] => Err(EmptyTargetResolution::NoMatch),
            _ => Err(EmptyTargetResolution::Ambiguous),
        }
    }
}

pub(crate) enum EmptyTargetResolution {
    Disabled,
    NoMatch,
    Ambiguous,
}

/// Invalid consumer Surface transfer policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceTransferPolicyError {
    detail: String,
}

impl SurfaceTransferPolicyError {
    /// Returns diagnostic detail.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SurfaceTransferPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for SurfaceTransferPolicyError {}

fn validate_empty_targets(
    policy: &EmptyDisplayProvisionPolicy,
) -> Result<(), SurfaceTransferPolicyError> {
    let EmptyDisplayProvisionPolicy::Enabled(targets) = policy else {
        return Ok(());
    };
    let mut displays = BTreeSet::new();
    let mut windows = BTreeSet::new();
    for target in targets {
        if target.display_bounds.size().is_empty() {
            return Err(invalid_policy(
                "empty-display target bounds have zero extent",
            ));
        }
        if target.placement.inner_size().is_empty() {
            return Err(invalid_policy(
                "empty-display target placement has zero extent",
            ));
        }
        if !displays.insert(target.display_id.clone()) {
            return Err(invalid_policy(format!(
                "duplicate empty-display target {}",
                target.display_id
            )));
        }
        if !windows.insert(target.window_id.clone()) {
            return Err(invalid_policy(format!(
                "duplicate provisioned window target {}",
                target.window_id
            )));
        }
    }
    Ok(())
}

fn invalid_policy(detail: impl Into<String>) -> SurfaceTransferPolicyError {
    SurfaceTransferPolicyError {
        detail: detail.into(),
    }
}
