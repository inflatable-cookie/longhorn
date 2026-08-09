use longhorn_core::{ScreenPoint, ScreenSize};

use super::{CaptureGeneration, FlushReason, MonotonicMillis};
use crate::{ApplyGeneration, WindowOperation, WindowOperationKind};

#[derive(Default)]
pub(super) struct WindowState {
    pub(super) last_input_at: Option<MonotonicMillis>,
    pub(super) apply: Option<ApplyExpectation>,
    pub(super) user_until: Option<MonotonicMillis>,
    pub(super) capture_generation: u64,
    pub(super) pending: Option<PendingCapture>,
}

pub(super) struct ApplyExpectation {
    pub(super) generation: ApplyGeneration,
    pub(super) expires_at: MonotonicMillis,
    pub(super) effects: Vec<ExpectedEffect>,
}

pub(super) struct PendingCapture {
    pub(super) generation: CaptureGeneration,
    pub(super) capture_due_at: MonotonicMillis,
    pub(super) capture_requested: bool,
    pub(super) captured: bool,
    pub(super) flush_due_at: Option<MonotonicMillis>,
    pub(super) force_flush: Option<FlushReason>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExpectedEffect {
    MoveExact(ScreenPoint, WindowOperationKind),
    MoveTransition(WindowOperationKind),
    ResizeExact(ScreenSize, WindowOperationKind),
    ResizeTransition(WindowOperationKind),
    ScaleTransition(WindowOperationKind),
    Close,
}

impl ExpectedEffect {
    pub(super) fn from_operation(operation: &WindowOperation) -> Vec<Self> {
        match operation {
            WindowOperation::Create { .. } => vec![
                Self::MoveTransition(operation.kind()),
                Self::ResizeTransition(operation.kind()),
                Self::ScaleTransition(operation.kind()),
            ],
            WindowOperation::Move { outer_origin, .. } => vec![
                Self::MoveExact(*outer_origin, operation.kind()),
                Self::ScaleTransition(operation.kind()),
            ],
            WindowOperation::Resize { inner_size, .. } => vec![
                Self::ResizeExact(*inner_size, operation.kind()),
                Self::ScaleTransition(operation.kind()),
            ],
            WindowOperation::Maximize { .. } | WindowOperation::Unmaximize { .. } => vec![
                Self::ResizeTransition(operation.kind()),
                Self::ScaleTransition(operation.kind()),
            ],
            WindowOperation::Close { .. } => vec![Self::Close],
            WindowOperation::Retag { .. }
            | WindowOperation::Show { .. }
            | WindowOperation::Hide { .. }
            | WindowOperation::Focus { .. } => Vec::new(),
        }
    }

    pub(super) fn matches_geometry(self, event: GeometryEvent) -> bool {
        match (self, event) {
            (Self::MoveExact(expected, _), GeometryEvent::Moved(actual)) => expected == actual,
            (Self::MoveTransition(_), GeometryEvent::Moved(_)) => true,
            (Self::ResizeExact(expected, _), GeometryEvent::Resized(actual)) => expected == actual,
            (Self::ResizeTransition(_), GeometryEvent::Resized(_)) => true,
            (Self::ScaleTransition(_), GeometryEvent::ScaleChanged) => true,
            _ => false,
        }
    }

    pub(super) const fn operation(self) -> Option<WindowOperationKind> {
        match self {
            Self::MoveExact(_, operation)
            | Self::MoveTransition(operation)
            | Self::ResizeExact(_, operation)
            | Self::ResizeTransition(operation)
            | Self::ScaleTransition(operation) => Some(operation),
            Self::Close => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum GeometryEvent {
    Moved(ScreenPoint),
    Resized(ScreenSize),
    ScaleChanged,
}
