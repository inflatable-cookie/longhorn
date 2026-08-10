import type {
  LayoutDocument,
  LayoutSchemaDefinition,
  PanelDefinition,
} from "@inflatable-cookie/longhorn/layout";
import type {
  PanelPresentationResolver,
  PoodleLayoutDefinitions,
} from "@inflatable-cookie/longhorn-poodle-svelte/poodle";
import type { SurfaceSnapshot } from "@inflatable-cookie/longhorn/surfaces";

const regionIds = [
  "primary",
  "secondary",
  "left",
  "right",
  "top",
  "bottom",
  "transport",
  "status",
];

export const schema: LayoutSchemaDefinition = {
  id: "schema:loophole",
  regions: regionIds.map((id, order) => ({
    id,
    family_id: "family:studio",
    order,
    empty_policy: "keep_visible",
    collapsible: id !== "primary",
  })),
  sizing_slots: [
    {
      id: "sizing:studio",
      order: 0,
      minimum: 200_000,
      default: 620_000,
      maximum: 800_000,
    },
  ],
};

const panel: PanelDefinition = {
  id: "panel:studio-tool",
  default_placements: [{ kind: "region", id: "primary" }],
  allowed_placements: [{ kind: "family", id: "family:studio" }],
  instance_policy: { kind: "multiple" },
  movable: true,
  closeable: true,
};

export const definitions: PoodleLayoutDefinitions = {
  schemas: [schema],
  panels: [panel],
};

export const layoutDocument: LayoutDocument = {
  revision: 12,
  containers: [
    {
      id: "container:arrangement",
      schema_id: schema.id,
      regions: schema.regions.map(({ id, collapsible }) => ({
        region_id: id,
        panel_instance_ids:
          id === "primary" ? ["panel-instance:mixer"] : [],
        active_panel_instance_id:
          id === "primary" ? "panel-instance:mixer" : null,
        collapsed: collapsible ? false : null,
      })),
      sizing_slots: [
        {
          sizing_slot_id: "sizing:studio",
          ratio: 620_000,
        },
      ],
    },
  ],
  panel_instances: [
    {
      id: "panel-instance:mixer",
      definition_id: panel.id,
    },
  ],
};

export const surfaceSnapshot: SurfaceSnapshot = {
  protocol_version: 1,
  epoch: 3,
  revision: 7,
  document: {
    revision: 7,
    surfaces: [
      {
        id: "surface:arrangement",
        layout_container_id: "container:arrangement",
        label: "Arrangement",
        presentation: { kind: "regional" },
        host_preferences: [
          {
            window_id: "window:studio",
            order: 0,
          },
        ],
      },
    ],
    windows: [
      {
        id: "window:studio",
        active_surface_id: "surface:arrangement",
      },
    ],
  },
};

export const resolvePanel: PanelPresentationResolver = () => ({
  label: "Mixer",
});
