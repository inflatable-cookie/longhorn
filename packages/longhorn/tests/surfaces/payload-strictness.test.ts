import { describe, expect, test } from "bun:test";
import type { EventTransport } from "@inflatable-cookie/longhorn/core";

import {
  assertValidSurfaceMutationCommand,
  assertValidSurfaceMutationResponse,
  assertValidSurfaceSnapshot,
  SurfaceClient,
} from "@inflatable-cookie/longhorn/surfaces";
import type {
  SurfaceDocument,
  SurfaceMutationCommand,
  SurfaceMutationOutcome,
  SurfaceMutationRequest,
  SurfaceMutationResponse,
} from "@inflatable-cookie/longhorn/surfaces";
import { expectStrict } from "../corruption.ts";

const document: SurfaceDocument = {
  revision: 2,
  surfaces: [
    {
      id: "surface:one",
      schema_id: "schema:main",
      label: "One",
      presentation: { kind: "focused_panel", panel_definition_id: "panel:console" },
      regions: [
        {
          region_id: "center",
          panel_instance_ids: ["instance:one"],
          active_panel_instance_id: "instance:one",
          collapsed: false,
        },
      ],
      sizing_slots: [{ sizing_slot_id: "slot:left", ratio: 500_000 }],
      host_preferences: [{ window_id: "window:main", order: 0 }],
    },
  ],
  panel_instances: [{ id: "instance:one", definition_id: "panel:console" }],
  windows: [{ id: "window:main", active_surface_id: "surface:one" }],
};

const surface_id = "surface:one";
const window_id = "window:main";
const presentation = { kind: "regional" } as const;
const commands: Array<SurfaceMutationCommand> = [
  {
    kind: "create_surface",
    surface_id,
    schema_id: "schema:main",
    label: "One",
    host_preferences: [{ window_id, order: 0 }],
  },
  { kind: "duplicate_surface", source_surface_id: surface_id, surface_id: "surface:two" },
  { kind: "rename_surface", surface_id, label: "Two" },
  { kind: "set_surface_presentation", surface_id, presentation },
  { kind: "activate_surface", window_id, surface_id },
  { kind: "reorder_window", window_id, surface_ids: [surface_id] },
  { kind: "move_surface", surface_id, target_window_id: window_id, insertion_index: 0 },
  { kind: "close_surface", surface_id },
];

const outcomes: Array<SurfaceMutationOutcome> = [
  { kind: "surface_created", surface_id },
  { kind: "surface_duplicated", source_surface_id: surface_id, surface_id: "surface:two" },
  { kind: "surface_renamed", surface_id },
  {
    kind: "surface_presentation_set",
    surface_id,
    presentation,
    previous_presentation: { kind: "focused_panel", panel_definition_id: "panel:console" },
  },
  { kind: "surface_activated", window_id, surface_id, previous_active_surface_id: surface_id },
  { kind: "window_reordered", window_id, surface_ids: [surface_id] },
  {
    kind: "surface_moved",
    surface_id,
    source_window_id: window_id,
    target_window_id: "window:other",
    insertion_index: 0,
  },
  { kind: "surface_closed", surface_id },
];

function committed(
  requestId: string,
  outcome: SurfaceMutationOutcome = { kind: "surface_closed", surface_id },
): SurfaceMutationResponse {
  return {
    status: "committed",
    receipt: {
      request_id: requestId,
      previous_revision: 1,
      committed_revision: 2,
      outcome,
      authoritative_document: document,
    },
  };
}

function rejected(requestId: string): SurfaceMutationResponse {
  return {
    status: "rejected",
    rejection: {
      request_id: requestId,
      current_revision: 2,
      code: "stale_revision",
      detail: "stale",
      authoritative_document: document,
    },
  };
}

describe("surfaces payload strictness", () => {
  test("refuses any corrupted member of a snapshot", () => {
    expectStrict(assertValidSurfaceSnapshot, {
      protocol_version: 1,
      epoch: 1,
      revision: 2,
      document,
    });
  });

  test("refuses any corrupted variant value in a command", () => {
    for (const command of commands) {
      expectStrict(assertValidSurfaceMutationCommand, command);
    }
  });

  test("refuses any corrupted receipt or rejection payload", () => {
    for (const outcome of outcomes) {
      expectStrict(assertValidSurfaceMutationResponse, committed("request:one", outcome));
    }
    expectStrict(assertValidSurfaceMutationResponse, rejected("request:one"));
  });
});

function answering(reply: SurfaceMutationResponse): SurfaceClient {
  const transport: EventTransport = {
    listen: async () => () => {},
    invoke: async () => reply,
  };
  return new SurfaceClient(transport);
}

describe("Surface mutation correlation", () => {
  const request: SurfaceMutationRequest = {
    request_id: "request:mine",
    expected_revision: 1,
    command: { kind: "close_surface", surface_id },
  };

  test("accepts a reply that echoes the request id", async () => {
    for (const reply of [committed("request:mine"), rejected("request:mine")]) {
      await expect(answering(reply).mutate(request)).resolves.toBe(reply);
    }
  });

  test("refuses a reply for another request", async () => {
    for (const reply of [committed("request:other"), rejected("request:other")]) {
      await expect(answering(reply).mutate(request)).rejects.toMatchObject({
        code: "uncorrelated_response",
      });
    }
  });
});
