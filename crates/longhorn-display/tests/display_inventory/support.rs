use std::{collections::VecDeque, convert::Infallible};

use longhorn_core::{DisplayId, ScaleFactor, ScreenPoint, ScreenRect, ScreenSize};
use longhorn_display::{
    AdapterDisplayKey, DisplayBuiltinStatus, DisplayEvidence, DisplayFacts, DisplayIdAllocator,
    DisplayLabel, KnownDisplay, KnownDisplayRegistry, ObservationId, ObservedDisplay,
    StrongDisplayKey, WeakDisplayKey,
};

pub(super) struct QueueAllocator {
    ids: VecDeque<DisplayId>,
    pub(super) calls: Vec<ObservationId>,
}

impl QueueAllocator {
    pub(super) fn new(ids: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            ids: ids
                .into_iter()
                .map(|id| DisplayId::new(id).unwrap())
                .collect(),
            calls: Vec::new(),
        }
    }
}

impl DisplayIdAllocator for QueueAllocator {
    type Error = Infallible;

    fn allocate(&mut self, observation: &ObservedDisplay) -> Result<DisplayId, Self::Error> {
        self.calls.push(observation.observation_id().clone());
        Ok(self.ids.pop_front().expect("fixture allocator exhausted"))
    }
}

pub(super) fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect::new(ScreenPoint::new(x, y), ScreenSize::new(width, height))
}

pub(super) fn facts(
    label: &str,
    x: i32,
    width: u32,
    height: u32,
    scale: u32,
    main: bool,
) -> DisplayFacts {
    DisplayFacts::new(
        Some(DisplayLabel::new(label).unwrap()),
        main,
        if main {
            DisplayBuiltinStatus::BuiltIn
        } else {
            DisplayBuiltinStatus::External
        },
        rect(x, 0, width, height),
        rect(x, 24, width, height - 24),
        ScaleFactor::from_thousandths(scale).unwrap(),
    )
}

pub(super) fn strong(namespace: &str, value: &str) -> StrongDisplayKey {
    StrongDisplayKey::new(namespace, value).unwrap()
}

pub(super) fn adapter(namespace: &str, value: &str) -> AdapterDisplayKey {
    AdapterDisplayKey::new(namespace, value).unwrap()
}

pub(super) fn weak(namespace: &str, value: &str) -> WeakDisplayKey {
    WeakDisplayKey::new(namespace, value).unwrap()
}

pub(super) fn known(id: &str, facts: DisplayFacts, evidence: DisplayEvidence) -> KnownDisplay {
    KnownDisplay::new(DisplayId::new(id).unwrap(), facts, evidence)
}

pub(super) fn observed(
    id: &str,
    facts: DisplayFacts,
    evidence: DisplayEvidence,
) -> ObservedDisplay {
    ObservedDisplay::new(ObservationId::new(id).unwrap(), facts, evidence)
}

pub(super) fn registry(displays: impl IntoIterator<Item = KnownDisplay>) -> KnownDisplayRegistry {
    KnownDisplayRegistry::from_displays(displays).unwrap()
}

pub(super) fn unavailable_allocator() -> QueueAllocator {
    QueueAllocator::new([])
}
