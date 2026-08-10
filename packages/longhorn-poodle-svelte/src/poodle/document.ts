import type {
  PanelInstance,
  PanelInstanceId,
  PlacementSelector,
  RegionDefinition,
  RegionId,
  RegionState,
} from "@inflatable-cookie/longhorn/layout";
import type {
  SurfaceRecord,
  SurfaceId,
  SurfaceDocument,
} from "@inflatable-cookie/longhorn/surfaces";

import { MissingLayoutMemberError } from "./types.ts";

export function findContainer(
  document: SurfaceDocument,
  containerId: SurfaceId,
): SurfaceRecord {
  const container = document.surfaces.find(({ id }) => id === containerId);
  if (!container) {
    throw new MissingLayoutMemberError("layout container", containerId);
  }
  return container;
}

export function findRegion(
  container: SurfaceRecord,
  regionId: RegionId,
): RegionState {
  const region = container.regions.find(
    ({ region_id }) => region_id === regionId,
  );
  if (!region) {
    throw new MissingLayoutMemberError("region state", regionId);
  }
  return region;
}

export function findPanelInstance(
  document: SurfaceDocument,
  panelInstanceId: PanelInstanceId,
): PanelInstance {
  const instance = document.panel_instances.find(
    ({ id }) => id === panelInstanceId,
  );
  if (!instance) {
    throw new MissingLayoutMemberError("panel instance", panelInstanceId);
  }
  return instance;
}

export function findPanelLocation(
  document: SurfaceDocument,
  panelInstanceId: PanelInstanceId,
): { container: SurfaceRecord; region: RegionState } {
  for (const container of document.surfaces) {
    for (const region of container.regions) {
      if (region.panel_instance_ids.includes(panelInstanceId)) {
        return { container, region };
      }
    }
  }
  throw new MissingLayoutMemberError("panel placement", panelInstanceId);
}

export function selectorMatches(
  selector: PlacementSelector,
  region: RegionDefinition,
): boolean {
  return selector.kind === "region"
    ? selector.id === region.id
    : selector.id === region.family_id;
}

export function updateRegion(
  document: SurfaceDocument,
  containerId: SurfaceId,
  regionId: RegionId,
  update: (region: RegionState) => RegionState,
): SurfaceDocument {
  return {
    ...document,
    surfaces: document.surfaces.map((container) =>
      container.id === containerId
        ? {
            ...container,
            regions: container.regions.map((region) =>
              region.region_id === regionId ? update(region) : region,
            ),
          }
        : container,
    ),
  };
}

export function removePanel(
  region: RegionState,
  panelInstanceId: PanelInstanceId,
): RegionState {
  const index = region.panel_instance_ids.indexOf(panelInstanceId);
  if (index < 0) return region;

  const panelInstanceIds = region.panel_instance_ids.filter(
    (id) => id !== panelInstanceId,
  );
  const activePanelInstanceId =
    region.active_panel_instance_id === panelInstanceId
      ? (panelInstanceIds[index] ?? panelInstanceIds.at(-1) ?? null)
      : region.active_panel_instance_id;
  return {
    ...region,
    panel_instance_ids: panelInstanceIds,
    active_panel_instance_id: activePanelInstanceId,
  };
}
