import type {
  LayoutContainer,
  LayoutContainerId,
  LayoutDocument,
  LayoutMutationCommand,
  LayoutMutationRequest,
  LayoutSchemaDefinition,
  PanelDefinition,
  PanelInstanceId,
  RegionVisibility,
  RegionDefinition,
  RegionId,
  RegionState,
  SizingSlotId,
} from "@inflatable-cookie/longhorn-layout";
import { projectOrdinaryRegionVisibility } from "@inflatable-cookie/longhorn-layout";

import {
  findContainer,
  findPanelInstance,
  findPanelLocation,
  findRegion,
  selectorMatches,
} from "./document.ts";
import {
  projectActivate,
  projectClose,
  projectCollapsed,
  projectMove,
  projectReorder,
  projectSizingSlot,
  type LayoutProjector,
} from "./projectors.ts";
import {
  MissingLayoutDocumentError,
  MissingLayoutMemberError,
  MissingPanelPresentationError,
  type PanelPresentationResolver,
  type PanelRenderContext,
  type PoodleLayoutBindingOptions,
  type RegionProjection,
  type SizingSlotProjection,
} from "./types.ts";

export type { PanelPresentation } from "./types.ts";

export class PoodleLayoutBinding {
  readonly #options: PoodleLayoutBindingOptions;
  readonly #schemas: ReadonlyMap<string, LayoutSchemaDefinition>;
  readonly #panels: ReadonlyMap<string, PanelDefinition>;
  #mutationTail = Promise.resolve();

  constructor(options: PoodleLayoutBindingOptions) {
    this.#options = options;
    this.#schemas = uniqueMap(options.definitions.schemas, "layout schema");
    this.#panels = uniqueMap(options.definitions.panels, "panel definition");
  }

  get document(): LayoutDocument {
    const document = this.#options.state.projected;
    if (!document) {
      throw new MissingLayoutDocumentError();
    }
    return document;
  }

  region(
    containerId: LayoutContainerId,
    regionId: RegionId,
    resolvePresentation: PanelPresentationResolver,
  ): RegionProjection {
    const document = this.document;
    const container = findContainer(document, containerId);
    const state = findRegion(container, regionId);
    const definition = this.#regionDefinition(container, regionId);
    const panels = new Map<PanelInstanceId, PanelRenderContext>();
    const items = state.panel_instance_ids.map((panelInstanceId) => {
      const instance = findPanelInstance(document, panelInstanceId);
      const panelDefinition = this.#panelDefinition(instance.definition_id);
      const presentation = resolvePresentation(instance);
      if (!presentation) {
        throw new MissingPanelPresentationError(instance.id);
      }
      if (presentation.label.trim().length === 0) {
        throw new TypeError(
          `panel presentation label is empty: ${panelInstanceId}`,
        );
      }
      panels.set(panelInstanceId, {
        instance,
        definition: panelDefinition,
        presentation,
      });
      return {
        value: panelInstanceId,
        label: presentation.label,
        icon: presentation.icon,
        closable: panelDefinition.closeable,
      };
    });

    let active: PanelRenderContext | null = null;
    if (state.active_panel_instance_id !== null) {
      active = panels.get(state.active_panel_instance_id) ?? null;
      if (!active) {
        throw new MissingLayoutMemberError(
          "active panel projection",
          state.active_panel_instance_id,
        );
      }
    }

    return {
      container,
      definition,
      state,
      items,
      panels,
      active,
    };
  }

  regionState(
    containerId: LayoutContainerId,
    regionId: RegionId,
  ): RegionState {
    return findRegion(findContainer(this.document, containerId), regionId);
  }

  regionDefinition(
    containerId: LayoutContainerId,
    regionId: RegionId,
  ): RegionDefinition {
    const container = findContainer(this.document, containerId);
    return this.#regionDefinition(container, regionId);
  }

  collapsibleRegionState(
    containerId: LayoutContainerId,
    regionId: RegionId,
  ): RegionState {
    const definition = this.regionDefinition(containerId, regionId);
    const state = this.regionState(containerId, regionId);
    if (!definition.collapsible || state.collapsed === null) {
      throw new TypeError(`region does not support collapse: ${regionId}`);
    }
    return state;
  }

  sizingSlot(
    containerId: LayoutContainerId,
    sizingSlotId: SizingSlotId,
  ): SizingSlotProjection {
    const container = findContainer(this.document, containerId);
    const schema = this.#schema(container.schema_id);
    const definition = schema.sizing_slots.find(
      ({ id }) => id === sizingSlotId,
    );
    const state = container.sizing_slots.find(
      ({ sizing_slot_id }) => sizing_slot_id === sizingSlotId,
    );
    if (!definition || !state) {
      throw new MissingLayoutMemberError("sizing slot", sizingSlotId);
    }
    return { definition, ratio: state.ratio };
  }

  canMove(
    panelInstanceId: PanelInstanceId,
    targetContainerId: LayoutContainerId,
    targetRegionId: RegionId,
  ): boolean {
    const document = this.document;
    const instance = findPanelInstance(document, panelInstanceId);
    const definition = this.#panelDefinition(instance.definition_id);
    if (!definition.movable) return false;

    const source = findPanelLocation(document, panelInstanceId);
    if (
      source.container.id === targetContainerId &&
      source.region.region_id === targetRegionId
    ) {
      return false;
    }

    const target = findContainer(document, targetContainerId);
    const targetRegion = this.#regionDefinition(target, targetRegionId);
    if (
      !definition.allowed_placements.some((selector) =>
        selectorMatches(selector, targetRegion),
      )
    ) {
      return false;
    }

    const peerCount = document.panel_instances.filter((candidate) => {
      if (
        candidate.id === panelInstanceId ||
        candidate.definition_id !== instance.definition_id
      ) {
        return false;
      }
      return findPanelLocation(document, candidate.id).container.id === target.id;
    }).length;

    switch (definition.instance_policy.kind) {
      case "one_per_container":
        return peerCount === 0;
      case "bounded":
        return peerCount < definition.instance_policy.maximum_per_container;
      case "singleton":
      case "multiple":
        return true;
    }
  }

  regionVisibilities(
    containerId: LayoutContainerId,
    movingPanelInstanceId?: PanelInstanceId,
  ): readonly RegionVisibility[] {
    const document = this.document;
    const container = findContainer(document, containerId);
    const schema = this.#schema(container.schema_id);

    return schema.regions.map((definition) => {
      const state = findRegion(container, definition.id);
      const ordinary = projectOrdinaryRegionVisibility(definition, state);
      if (
        ordinary.state === "hidden" &&
        movingPanelInstanceId !== undefined &&
        this.canMove(movingPanelInstanceId, containerId, definition.id)
      ) {
        return {
          region_id: definition.id,
          state: "transiently_revealed",
        };
      }
      return ordinary;
    });
  }

  activate(panelInstanceId: PanelInstanceId): void {
    this.#submit(
      { kind: "activate_panel", panel_instance_id: panelInstanceId },
      projectActivate(panelInstanceId),
    );
  }

  close(panelInstanceId: PanelInstanceId): void {
    this.#submit(
      { kind: "close_panel", panel_instance_id: panelInstanceId },
      projectClose(panelInstanceId),
    );
  }

  reorder(
    containerId: LayoutContainerId,
    regionId: RegionId,
    panelInstanceIds: readonly PanelInstanceId[],
  ): void {
    this.#submit(
      {
        kind: "reorder_region",
        container_id: containerId,
        region_id: regionId,
        panel_instance_ids: [...panelInstanceIds],
      },
      projectReorder(containerId, regionId, panelInstanceIds),
    );
  }

  move(
    panelInstanceId: PanelInstanceId,
    targetContainerId: LayoutContainerId,
    targetRegionId: RegionId,
    insertionIndex: number,
  ): void {
    this.#submit(
      {
        kind: "move_panel",
        panel_instance_id: panelInstanceId,
        target_container_id: targetContainerId,
        target_region_id: targetRegionId,
        insertion_index: insertionIndex,
      },
      projectMove(
        panelInstanceId,
        targetContainerId,
        targetRegionId,
        insertionIndex,
      ),
    );
  }

  setCollapsed(
    containerId: LayoutContainerId,
    regionId: RegionId,
    collapsed: boolean,
  ): void {
    this.#submit(
      {
        kind: "set_region_collapsed",
        container_id: containerId,
        region_id: regionId,
        collapsed,
      },
      projectCollapsed(containerId, regionId, collapsed),
    );
  }

  setSizingSlot(
    containerId: LayoutContainerId,
    sizingSlotId: SizingSlotId,
    ratio: number,
  ): void {
    this.#submit(
      {
        kind: "set_sizing_slot",
        container_id: containerId,
        sizing_slot_id: sizingSlotId,
        ratio,
      },
      projectSizingSlot(containerId, sizingSlotId, ratio),
    );
  }

  #submit(
    command: LayoutMutationCommand,
    project: LayoutProjector,
  ): void {
    this.#mutationTail = this.#mutationTail.then(async () => {
      const request: LayoutMutationRequest = {
        request_id: this.#options.nextRequestId(),
        expected_revision: this.document.revision,
        command,
      };
      const result = await this.#options.state.dispatch(request, project);
      this.#options.onResult?.(result);
    }).catch((error) => {
      try {
        this.#options.onError(error);
      } catch {
        // Error reporting cannot poison later queued mutations.
      }
    });
  }

  #schema(schemaId: string): LayoutSchemaDefinition {
    const schema = this.#schemas.get(schemaId);
    if (!schema) {
      throw new MissingLayoutMemberError("layout schema", schemaId);
    }
    return schema;
  }

  #regionDefinition(
    container: LayoutContainer,
    regionId: RegionId,
  ): RegionDefinition {
    const definition = this.#schema(container.schema_id).regions.find(
      ({ id }) => id === regionId,
    );
    if (!definition) {
      throw new MissingLayoutMemberError("region definition", regionId);
    }
    return definition;
  }

  #panelDefinition(panelDefinitionId: string): PanelDefinition {
    const definition = this.#panels.get(panelDefinitionId);
    if (!definition) {
      throw new MissingLayoutMemberError(
        "panel definition",
        panelDefinitionId,
      );
    }
    return definition;
  }
}

export function createPoodleLayoutBinding(
  options: PoodleLayoutBindingOptions,
): PoodleLayoutBinding {
  return new PoodleLayoutBinding(options);
}

function uniqueMap<T extends { readonly id: string }>(
  values: readonly T[],
  kind: string,
): ReadonlyMap<string, T> {
  const result = new Map<string, T>();
  for (const value of values) {
    if (result.has(value.id)) {
      throw new TypeError(`duplicate ${kind}: ${value.id}`);
    }
    result.set(value.id, value);
  }
  return result;
}
