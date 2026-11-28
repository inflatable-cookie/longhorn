import { describe, test } from "bun:test";

import {
  assertValidSurfaceSessionResponse,
  assertValidSurfaceTransferResponse,
} from "@inflatable-cookie/longhorn/surface-transfer";
import type {
  SurfaceTransferAbort,
  SurfaceTransferCompletion,
  SurfaceTransferTarget,
} from "@inflatable-cookie/longhorn/surface-transfer";
import { expectStrict } from "../corruption.ts";

const session_id = "cd".repeat(16);

const abort: SurfaceTransferAbort = {
  protocol_version: 1,
  request_id: "request:abort",
  source: { domain: "surface_transfer", code: "surface_mutation_rejected" },
  surface_code: "stale_revision",
  message: "refused",
  retryable: false,
  session_consumed: true,
  reconciliation_required: false,
};

const targets: Array<SurfaceTransferTarget> = [
  {
    kind: "existing",
    target: {
      path: "screen_point",
      window_id: "window:target",
      drop_zone_id: "zone:tabs",
      insertion_position: 0,
      binding: {
        kind: "surface_window",
        host_binding_id: "binding:target",
        document_id: "app.surfaces",
        revision: 3,
      },
    },
  },
  {
    kind: "provisioned",
    drop_point: { x: 1200, y: -40 },
    provisioning: {
      window_id: "window:new",
      host_binding_id: "binding:new",
      display_id: "display:two",
    },
  },
];

function completion(target: SurfaceTransferTarget): SurfaceTransferCompletion {
  return {
    protocol_version: 1,
    request_id: "request:commit",
    session_id,
    source_host_binding_id: "binding:source",
    target_host_binding_id: "binding:target",
    previous_revision: 3,
    committed_revision: 4,
    authoritative_document: {
      revision: 4,
      surfaces: [],
      panel_instances: [],
      windows: [{ id: "window:target", active_surface_id: "surface:one" }],
    },
    target,
  };
}

describe("surface-transfer payload strictness", () => {
  test("refuses any corrupted session or abort payload", () => {
    expectStrict(assertValidSurfaceSessionResponse, {
      status: "started",
      session: {
        protocol_version: 1,
        request_id: "request:start",
        payload: { protocol_version: 1, session_id },
      },
    });
    expectStrict(assertValidSurfaceSessionResponse, { status: "aborted", abort });
    expectStrict(assertValidSurfaceTransferResponse, { status: "aborted", abort });
  });

  test("refuses any corrupted completion payload", () => {
    for (const target of targets) {
      expectStrict(assertValidSurfaceTransferResponse, {
        status: "committed",
        completion: completion(target),
      });
    }
  });
});
