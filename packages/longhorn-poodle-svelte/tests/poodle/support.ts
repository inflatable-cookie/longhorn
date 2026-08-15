import { readFileSync } from "node:fs";
import { URL as NodeURL, fileURLToPath } from "node:url";

import type {
  LayoutMutationRequest,
  LayoutSchemaDefinition,
  PanelDefinition,
  PanelInstance,
} from "@inflatable-cookie/longhorn/layout";
import type {
  SurfaceDocument,
} from "@inflatable-cookie/longhorn/surfaces";
import { LayoutState, type LayoutDispatchResult } from "@inflatable-cookie/longhorn-poodle-svelte/layout";

import {
  createPoodleLayoutBinding,
  type PanelPresentationResolver,
  type PoodleLayoutBinding,
  type PoodleLayoutDefinitions,
} from "../../src/poodle/index.ts";

export interface Deferred<T> {
  readonly promise: Promise<T>;
  resolve(value: T): void;
}

export function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

export function loadShape(name: "window-bound" | "surface-bound"): Shape {
  // Node's URL, not the global: happy-dom shadows `URL` with a window
  // implementation that resolves a file-scheme base against the page location.
  const path = fileURLToPath(
    new NodeURL(
      `../../../../fixtures/layout/${name}-conformance-v1.json`,
      import.meta.url,
    ),
  );
  const fixture = JSON.parse(readFileSync(path, "utf8")) as {
    definitions: {
      schema: LayoutSchemaDefinition;
      panels: PanelDefinition[];
    };
  };
  return {
    definitions: {
      schemas: [fixture.definitions.schema],
      panels: fixture.definitions.panels,
    },
    schema: fixture.definitions.schema,
  };
}

export function shapeDocument(
  shape: Shape,
  placements: Readonly<Record<string, readonly PanelInstance[]>>,
  revision = 1,
): SurfaceDocument {
  const panelInstances = Object.values(placements).flat();
  return {
    revision,
    surfaces: [
      {
        id: "surface:primary",
        schema_id: shape.schema.id,
        label: null,
        presentation: { kind: "regional" } as const,
        regions: shape.schema.regions.map((definition) => {
          const instances = placements[definition.id] ?? [];
          return {
            region_id: definition.id,
            panel_instance_ids: instances.map(({ id }) => id),
            active_panel_instance_id: instances[0]?.id ?? null,
            collapsed: definition.collapsible ? false : null,
          };
        }),
        sizing_slots: shape.schema.sizing_slots.map((definition) => ({
          sizing_slot_id: definition.id,
          ratio: definition.default,
        })),
        host_preferences: [],
      },
    ],
    panel_instances: panelInstances,
    windows: [],
  };
}

export function instance(id: string): PanelInstance {
  return { id, definition_id: "panel:workspace-tool" };
}

export const resolvePanel: PanelPresentationResolver = (panel) => ({
  label: panel.id.replace("instance:", "").toUpperCase(),
});

export function mountedBinding(
  definitions: PoodleLayoutDefinitions,
  document: SurfaceDocument,
  dispatch: (request: LayoutMutationRequest) => Promise<LayoutDispatchResult>,
): {
  readonly binding: PoodleLayoutBinding;
  readonly state: LayoutState;
  readonly errors: unknown[];
} {
  const state = new LayoutState({ dispatch });
  const errors: unknown[] = [];
  let requestNumber = 0;
  void state.start();
  state.accept(document);
  return {
    state,
    errors,
    binding: createPoodleLayoutBinding({
      state,
      definitions,
      nextRequestId: () => `request:poodle-${++requestNumber}`,
      onError: (error) => errors.push(error),
    }),
  };
}

export function rejected(
  request: LayoutMutationRequest,
  document: SurfaceDocument,
): LayoutDispatchResult {
  return {
    status: "rejected",
    rejection: {
      request_id: request.request_id,
      current_revision: document.revision,
      code: "stale_revision",
      detail: "newer authority",
      authoritative_document: document,
    },
  };
}

export interface Shape {
  readonly definitions: PoodleLayoutDefinitions;
  readonly schema: LayoutSchemaDefinition;
}
