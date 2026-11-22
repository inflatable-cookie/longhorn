//! Shared attach-generation admission rule for mechanism adapters.
//!
//! Contract 017 states the rule once: reject stale, future, retired,
//! attaching, or absent generations before native work. This module owns the
//! comparisons and the admission order so a rule fix lands in one file.
//! Mechanism adapters keep their own state and error enums and map
//! [`GenerationRejection`] into them.
//!
//! Two extensions stay mechanism-specific:
//!
//! - backing-surface storage can outlive host invalidation inside the
//!   invalidate-then-detach window, so that adapter additionally rejects a
//!   retained attachment's invalidated generation;
//! - an isolated-window owner can fail terminally while its island lives on,
//!   so that adapter additionally rejects the failed generation and yields
//!   only to exactly the next one.

use std::{error::Error, fmt};

use crate::AttachGeneration;

/// Mechanism-neutral rejection of a supplied attach generation.
///
/// One variant per clause of the contract 017 rule. Adapters map each
/// variant onto their own mechanism-shaped error enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationRejection {
    /// Evidence or work names an older generation.
    Stale {
        /// Current adapter generation.
        current: AttachGeneration,
        /// Supplied older generation.
        supplied: AttachGeneration,
    },
    /// Work names a generation beyond the next legal attachment.
    Future {
        /// Current adapter generation.
        current: AttachGeneration,
        /// Supplied future generation.
        supplied: AttachGeneration,
    },
    /// A new generation was requested while the current one remained live.
    Attached(AttachGeneration),
    /// A completed or invalidated generation cannot attach again.
    Retired(AttachGeneration),
    /// No attachment exists for an operation that requires one.
    Absent,
    /// The generation is reserved but native attachment has not completed.
    Attaching,
}

impl fmt::Display for GenerationRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stale { current, supplied } => write!(
                formatter,
                "stale generation {}; current is {}",
                supplied.get(),
                current.get()
            ),
            Self::Future { current, supplied } => write!(
                formatter,
                "future generation {}; current is {}",
                supplied.get(),
                current.get()
            ),
            Self::Attached(generation) => {
                write!(
                    formatter,
                    "generation {} remains attached",
                    generation.get()
                )
            }
            Self::Retired(generation) => {
                write!(formatter, "generation {} is retired", generation.get())
            }
            Self::Absent => formatter.write_str("no attachment is current"),
            Self::Attaching => formatter.write_str("attach is in progress"),
        }
    }
}

impl Error for GenerationRejection {}

/// Snapshot of one current attachment for generation admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttachmentGate {
    generation: AttachGeneration,
    complete: bool,
}

impl AttachmentGate {
    /// Records one current attachment and whether native attach completed.
    #[must_use]
    pub const fn new(generation: AttachGeneration, complete: bool) -> Self {
        Self {
            generation,
            complete,
        }
    }

    /// Returns the attached generation.
    #[must_use]
    pub const fn generation(self) -> AttachGeneration {
        self.generation
    }

    /// Returns whether native attachment completed for this generation.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        self.complete
    }
}

/// Rejects a supplied generation that is not exactly the latest.
pub fn compare_generation(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), GenerationRejection> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        Err(GenerationRejection::Stale { current, supplied })
    } else if supplied > current {
        Err(GenerationRejection::Future { current, supplied })
    } else {
        Ok(())
    }
}

/// Rejects a supplied generation that is neither the latest nor the next.
pub fn compare_generation_allow_next(
    current: Option<AttachGeneration>,
    supplied: AttachGeneration,
) -> Result<(), GenerationRejection> {
    let Some(current) = current else {
        return Ok(());
    };
    if supplied < current {
        return Err(GenerationRejection::Stale { current, supplied });
    }
    if supplied == current || current.checked_next().ok() == Some(supplied) {
        Ok(())
    } else {
        Err(GenerationRejection::Future { current, supplied })
    }
}

/// Requires the supplied generation to match the attached generation.
pub fn compare_attached_generation(
    current: AttachGeneration,
    supplied: AttachGeneration,
) -> Result<(), GenerationRejection> {
    compare_generation(Some(current), supplied)
}

/// Validates a plan's generation against current adapter authority.
///
/// With a live attachment, the plan must name exactly the attached
/// generation. Without one, the plan must name the latest or next
/// generation, and a retired generation cannot reattach.
pub fn validate_plan_generation(
    latest: Option<AttachGeneration>,
    retired: Option<AttachGeneration>,
    attached: Option<AttachGeneration>,
    supplied: AttachGeneration,
    includes_attach: bool,
) -> Result<(), GenerationRejection> {
    if let Some(attached) = attached {
        if supplied < attached {
            return Err(GenerationRejection::Stale {
                current: attached,
                supplied,
            });
        }
        if supplied > attached {
            return Err(GenerationRejection::Attached(attached));
        }
    } else {
        compare_generation_allow_next(latest, supplied)?;
        if retired == Some(supplied) && includes_attach {
            return Err(GenerationRejection::Retired(supplied));
        }
    }
    Ok(())
}

/// Validates one attach reservation for the supplied generation.
///
/// Returns `Ok(true)` when the generation already completed attach and the
/// call is an idempotent replay, and `Ok(false)` when the caller must
/// reserve the generation. A reserved or live attachment rejects any new
/// generation until it detaches.
pub fn check_attach_reservation(
    latest: Option<AttachGeneration>,
    retired: Option<AttachGeneration>,
    attached: Option<AttachmentGate>,
    supplied: AttachGeneration,
) -> Result<bool, GenerationRejection> {
    if let Some(gate) = attached {
        if gate.generation() == supplied && gate.is_complete() {
            return Ok(true);
        }
        return Err(GenerationRejection::Attached(gate.generation()));
    }
    compare_generation_allow_next(latest, supplied)?;
    if retired == Some(supplied) {
        return Err(GenerationRejection::Retired(supplied));
    }
    Ok(false)
}

/// Returns the current attachment when it belongs to the supplied generation.
pub fn gate_attached<T>(
    latest: Option<AttachGeneration>,
    retired: Option<AttachGeneration>,
    attached: Option<T>,
    supplied: AttachGeneration,
    generation: impl FnOnce(&T) -> AttachGeneration,
) -> Result<T, GenerationRejection> {
    compare_generation(latest, supplied)?;
    if retired == Some(supplied) {
        return Err(GenerationRejection::Retired(supplied));
    }
    let attached = attached.ok_or(GenerationRejection::Absent)?;
    compare_attached_generation(generation(&attached), supplied)?;
    Ok(attached)
}

/// Returns the complete attachment to detach, or `None` if already retired.
///
/// Call after [`compare_generation`]. Unlike the regular attachment gate,
/// detach admits a retained attachment that still needs native teardown.
pub fn gate_detach<T>(
    retired: Option<AttachGeneration>,
    attached: Option<T>,
    supplied: AttachGeneration,
    inspect: impl FnOnce(&T) -> AttachmentGate,
) -> Result<Option<T>, GenerationRejection> {
    let Some(attached) = attached else {
        return if retired == Some(supplied) {
            Ok(None)
        } else {
            Err(GenerationRejection::Absent)
        };
    };
    let gate = inspect(&attached);
    compare_attached_generation(gate.generation(), supplied)?;
    if !gate.is_complete() {
        return Err(GenerationRejection::Attaching);
    }
    Ok(Some(attached))
}
