import type {
  LayoutSchemaDefinition,
  PanelDefinition,
} from "@inflatable-cookie/longhorn/layout";
import type {
  SurfaceDocument,
} from "@inflatable-cookie/longhorn/surfaces";
import type {
  PanelPresentationResolver,
  PoodleLayoutDefinitions,
} from "@inflatable-cookie/longhorn-poodle-svelte/poodle";

const regionIds = ["navigation", "context", "main", "inspector", "console"];

export const schema: LayoutSchemaDefinition = {
  id: "schema:nucleus",
  regions: regionIds.map((id, order) => ({
    id,
    family_id: "family:workspace",
    order,
    empty_policy: "keep_visible",
    collapsible: id !== "main",
  })),
  sizing_slots: [
    {
      id: "sizing:navigation-main",
      order: 0,
      minimum: 150_000,
      default: 260_000,
      maximum: 450_000,
    },
  ],
};

const panel: PanelDefinition = {
  id: "panel:workspace",
  default_placements: [{ kind: "region", id: "main" }],
  allowed_placements: [{ kind: "family", id: "family:workspace" }],
  instance_policy: { kind: "multiple" },
  movable: true,
  closeable: true,
};

export const definitions: PoodleLayoutDefinitions = {
  schemas: [schema],
  panels: [panel],
};

export const document: SurfaceDocument = {
  revision: 4,
  surfaces: [
    {
      id: "surface:nucleus",
      schema_id: schema.id,
      label: null,
      presentation: { kind: "regional" },
      host_preferences: [],
      regions: schema.regions.map(({ id, collapsible }) => ({
        region_id: id,
        panel_instance_ids: id === "main" ? ["panel-instance:project"] : [],
        active_panel_instance_id:
          id === "main" ? "panel-instance:project" : null,
        collapsed: collapsible ? false : null,
      })),
      sizing_slots: [
        {
          sizing_slot_id: "sizing:navigation-main",
          ratio: 260_000,
        },
      ],
    },
  ],
  panel_instances: [
    {
      id: "panel-instance:project",
      definition_id: panel.id,
    },
  ],
  windows: [],
};

export const resolvePanel: PanelPresentationResolver = () => ({
  label: "Project",
});
