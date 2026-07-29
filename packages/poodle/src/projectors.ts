import type {
  LayoutContainerId,
  LayoutDocument,
  PanelInstanceId,
  RegionId,
  SizingSlotId,
} from "@longhorn/layout";

import { findPanelLocation, removePanel, updateRegion } from "./document.ts";

export type LayoutProjector = (document: LayoutDocument) => LayoutDocument;

export function projectActivate(
  panelInstanceId: PanelInstanceId,
): LayoutProjector {
  return (document) => {
    const location = findPanelLocation(document, panelInstanceId);
    return updateRegion(
      document,
      location.container.id,
      location.region.region_id,
      (region) => ({
        ...region,
        active_panel_instance_id: panelInstanceId,
      }),
    );
  };
}

export function projectClose(
  panelInstanceId: PanelInstanceId,
): LayoutProjector {
  return (document) => {
    const location = findPanelLocation(document, panelInstanceId);
    return {
      ...updateRegion(
        document,
        location.container.id,
        location.region.region_id,
        (region) => removePanel(region, panelInstanceId),
      ),
      panel_instances: document.panel_instances.filter(
        ({ id }) => id !== panelInstanceId,
      ),
    };
  };
}

export function projectReorder(
  containerId: LayoutContainerId,
  regionId: RegionId,
  panelInstanceIds: readonly PanelInstanceId[],
): LayoutProjector {
  const order = [...panelInstanceIds];
  return (document) =>
    updateRegion(document, containerId, regionId, (region) => ({
      ...region,
      panel_instance_ids: order,
    }));
}

export function projectMove(
  panelInstanceId: PanelInstanceId,
  targetContainerId: LayoutContainerId,
  targetRegionId: RegionId,
  insertionIndex: number,
): LayoutProjector {
  return (document) => {
    const source = findPanelLocation(document, panelInstanceId);
    const withoutSource = updateRegion(
      document,
      source.container.id,
      source.region.region_id,
      (region) => removePanel(region, panelInstanceId),
    );
    return updateRegion(
      withoutSource,
      targetContainerId,
      targetRegionId,
      (region) => {
        const panelInstanceIds = [...region.panel_instance_ids];
        panelInstanceIds.splice(insertionIndex, 0, panelInstanceId);
        return {
          ...region,
          panel_instance_ids: panelInstanceIds,
          active_panel_instance_id: panelInstanceId,
        };
      },
    );
  };
}

export function projectCollapsed(
  containerId: LayoutContainerId,
  regionId: RegionId,
  collapsed: boolean,
): LayoutProjector {
  return (document) =>
    updateRegion(document, containerId, regionId, (region) => ({
      ...region,
      collapsed,
    }));
}

export function projectSizingSlot(
  containerId: LayoutContainerId,
  sizingSlotId: SizingSlotId,
  ratio: number,
): LayoutProjector {
  return (document) => ({
    ...document,
    containers: document.containers.map((container) =>
      container.id === containerId
        ? {
            ...container,
            sizing_slots: container.sizing_slots.map((slot) =>
              slot.sizing_slot_id === sizingSlotId ? { ...slot, ratio } : slot,
            ),
          }
        : container,
    ),
  });
}
