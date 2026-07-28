use std::time::Duration;

use longhorn_config::{
    ConfigDomain, DebounceClock, DebouncePolicy, DebounceStrategy, DomainDescriptor,
    DomainFilePath, DomainIssue, DurabilityRequirement, LoadOutcome, MigrationStep,
    MutationOptions, StorageClass,
};
use longhorn_core::{DomainId, SchemaVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct Layout {
    pub(super) sidebar_width: u32,
    pub(super) active_panel: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct Geometry {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct Presentation {
    pub(super) navigation_percent: u8,
    pub(super) selected_node: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct DesktopState {
    pub(super) layout: Layout,
    pub(super) geometry: Geometry,
    pub(super) presentation: Presentation,
    pub(super) theme: String,
}

pub(super) struct DesktopDomain {
    descriptor: DomainDescriptor,
}

impl DesktopDomain {
    pub(super) fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new("example.desktop-state").unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::MachineState,
                Some(DomainFilePath::new("desktop/state.json").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for DesktopDomain {
    type Value = DesktopState;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }

    fn default_value(&self) -> Self::Value {
        DesktopState {
            layout: Layout {
                sidebar_width: 240,
                active_panel: "files".to_owned(),
            },
            geometry: Geometry {
                x: 0,
                y: 0,
                width: 1200,
                height: 800,
            },
            presentation: Presentation {
                navigation_percent: 24,
                selected_node: "workspace".to_owned(),
            },
            theme: "light".to_owned(),
        }
    }

    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        serde_json::from_value(value).map_err(|error| DomainIssue::new("decode", error.to_string()))
    }

    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        serde_json::to_value(value).map_err(|error| DomainIssue::new("encode", error.to_string()))
    }

    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        if value.layout.active_panel.is_empty() || value.presentation.selected_node.is_empty() {
            Err(DomainIssue::new("empty-id", "state ids cannot be empty"))
        } else {
            Ok(())
        }
    }

    fn validate_raw(
        &self,
        _schema_version: SchemaVersion,
        value: &Value,
    ) -> Result<(), DomainIssue> {
        self.decode(value.clone())
            .and_then(|decoded| self.validate(&decoded))
    }

    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

#[derive(Clone, Default)]
pub(super) struct DesktopPatch {
    pub(super) sidebar_width: Option<u32>,
    pub(super) active_panel: Option<String>,
    pub(super) geometry: Option<Geometry>,
    pub(super) presentation: Option<Presentation>,
}

pub(super) struct DesktopStrategy;

impl DebounceStrategy<DesktopDomain> for DesktopStrategy {
    type Intent = DesktopPatch;

    fn coalesce(
        &self,
        previous: &Self::Intent,
        next: Self::Intent,
    ) -> Result<Self::Intent, DomainIssue> {
        Ok(DesktopPatch {
            sidebar_width: next.sidebar_width.or(previous.sidebar_width),
            active_panel: next.active_panel.or_else(|| previous.active_panel.clone()),
            geometry: next.geometry.or_else(|| previous.geometry.clone()),
            presentation: next.presentation.or_else(|| previous.presentation.clone()),
        })
    }

    fn apply(&self, intent: &Self::Intent, value: &mut DesktopState) -> Result<(), DomainIssue> {
        if let Some(width) = intent.sidebar_width {
            value.layout.sidebar_width = width;
        }
        if let Some(panel) = &intent.active_panel {
            value.layout.active_panel.clone_from(panel);
        }
        if let Some(geometry) = &intent.geometry {
            value.geometry.clone_from(geometry);
        }
        if let Some(presentation) = &intent.presentation {
            value.presentation.clone_from(presentation);
        }
        Ok(())
    }

    fn pending_weight(&self, intent: &Self::Intent) -> usize {
        usize::from(intent.sidebar_width.is_some()) * size_of::<u32>()
            + intent.active_panel.as_ref().map_or(0, String::len)
            + usize::from(intent.geometry.is_some()) * size_of::<Geometry>()
            + intent
                .presentation
                .as_ref()
                .map_or(0, |value| size_of::<u8>() + value.selected_node.len())
    }
}

#[derive(Clone, Copy)]
pub(super) struct FixedClock;

impl DebounceClock for FixedClock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}

pub(super) fn policy() -> DebouncePolicy {
    DebouncePolicy::new(
        Duration::from_millis(200),
        256,
        MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic),
    )
    .unwrap()
}

pub(super) fn loaded(store: &longhorn_config::ConfigStore, domain: &DesktopDomain) -> DesktopState {
    let LoadOutcome::Ready(loaded) = store.load(domain).unwrap() else {
        panic!("expected ready desktop state");
    };
    loaded.value
}
