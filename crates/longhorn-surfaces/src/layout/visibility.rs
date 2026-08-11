use std::{error::Error, fmt};

use longhorn_core::{PanelDefinitionId, RegionId, SurfaceId};
use serde::{Deserialize, Serialize};

use crate::layout::validation::validate_document;
use crate::{EmptyRegionPolicy, LayoutDefinitionRegistry, LayoutValidationError, SurfaceDocument};

/// Projected presentation state for one semantic region.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RegionVisibilityState {
    /// Occupancy or region policy keeps the region in normal presentation.
    Visible,
    /// An empty region is absent from normal presentation.
    Hidden,
    /// A movable panel makes an otherwise hidden eligible target visible.
    TransientlyRevealed,
}

/// Visibility projection for one semantic region.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct RegionVisibility {
    region_id: RegionId,
    state: RegionVisibilityState,
}

impl RegionVisibility {
    /// Returns stable semantic region identity.
    #[must_use]
    pub const fn region_id(&self) -> &RegionId {
        &self.region_id
    }

    /// Returns projected visibility state.
    #[must_use]
    pub const fn state(&self) -> RegionVisibilityState {
        self.state
    }
}

/// Projects normal or transient-reveal visibility without mutating state.
pub fn project_region_visibility(
    registry: &LayoutDefinitionRegistry,
    document: &SurfaceDocument,
    surface_id: &SurfaceId,
    moving_panel_definition_id: Option<&PanelDefinitionId>,
) -> Result<Vec<RegionVisibility>, VisibilityProjectionError> {
    validate_document(registry, document).map_err(VisibilityProjectionError::InvalidDocument)?;
    let surface = document
        .surface(surface_id)
        .ok_or_else(|| VisibilityProjectionError::UnknownSurface(surface_id.clone()))?;
    let schema = registry
        .schema(surface.schema_id())
        .expect("schema existence was validated");
    let moving_panel = moving_panel_definition_id
        .map(|id| {
            registry
                .panel_definition(id)
                .ok_or_else(|| VisibilityProjectionError::UnknownPanelDefinition(id.clone()))
        })
        .transpose()?;

    let mut projection = Vec::with_capacity(schema.regions().len());
    for definition in schema.regions() {
        let state = surface
            .region(definition.id())
            .expect("complete region state was validated");
        let visible = !state.panel_instance_ids().is_empty()
            || definition.empty_policy() == EmptyRegionPolicy::KeepVisible;
        let reveal = !visible
            && moving_panel.is_some_and(|panel| {
                panel.is_movable()
                    && registry
                        .is_panel_allowed_in(surface.schema_id(), panel.id(), definition.id())
                        .unwrap_or(false)
            });

        projection.push(RegionVisibility {
            region_id: definition.id().clone(),
            state: if visible {
                RegionVisibilityState::Visible
            } else if reveal {
                RegionVisibilityState::TransientlyRevealed
            } else {
                RegionVisibilityState::Hidden
            },
        });
    }
    Ok(projection)
}

/// Visibility projection failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VisibilityProjectionError {
    /// The source document was invalid.
    InvalidDocument(LayoutValidationError),
    /// The requested layout surface did not exist.
    UnknownSurface(SurfaceId),
    /// The transient panel definition was not registered.
    UnknownPanelDefinition(PanelDefinitionId),
}

impl fmt::Display for VisibilityProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocument(error) => write!(formatter, "invalid layout document: {error}"),
            Self::UnknownSurface(id) => write!(formatter, "unknown layout surface {id}"),
            Self::UnknownPanelDefinition(id) => {
                write!(formatter, "unknown panel definition {id}")
            }
        }
    }
}

impl Error for VisibilityProjectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDocument(error) => Some(error),
            Self::UnknownSurface(_) | Self::UnknownPanelDefinition(_) => None,
        }
    }
}
