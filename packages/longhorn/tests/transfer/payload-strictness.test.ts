import { describe, test } from "bun:test";

import {
  assertValidPanelTransferResponse,
  assertValidTransferCancelResponse,
  assertValidTransferClientSnapshot,
  assertValidTransferCommitSelector,
  assertValidTransferLeaseResponse,
  assertValidTransferSessionResponse,
  assertValidTransferTargetBinding,
} from "@inflatable-cookie/longhorn/transfer";
import type {
  PanelTransferResponse,
  TransferAbort,
  TransferCommittedTarget,
  TransferTargetBinding,
} from "@inflatable-cookie/longhorn/transfer";
import { expectStrict } from "../corruption.ts";

const session_id = "ab".repeat(16);
const bindings: Array<TransferTargetBinding> = [
  {
    kind: "panel_region",
    host_binding_id: "binding:target",
    document_id: "app.layout",
    revision: 7,
    surface_id: "surface:target",
    region_id: "center",
  },
  {
    kind: "surface_window",
    host_binding_id: "binding:surface",
    document_id: "app.surfaces",
    revision: 9,
  },
];

const target: TransferCommittedTarget = {
  path: "explicit_zone",
  window_id: "window:target",
  drop_zone_id: "zone:center",
  insertion_position: 1,
  binding: bindings[0]!,
};

const abort: TransferAbort = {
  protocol_version: 1,
  request_id: "request:abort",
  source: { domain: "panel", code: "layout_mutation_rejected" },
  layout_code: "stale_revision",
  message: "refused",
  retryable: false,
  session_consumed: true,
};

const committed: PanelTransferResponse = {
  status: "committed",
  completion: {
    protocol_version: 1,
    request_id: "request:commit",
    session_id,
    source_host_binding_id: "binding:source",
    target_host_binding_id: "binding:target",
    source_binding_kind: "direct_window",
    target_binding_kind: "surface_container",
    previous_revision: 7,
    committed_revision: 8,
    authoritative_document: {
      revision: 8,
      surfaces: [],
      panel_instances: [{ id: "instance:one", definition_id: "panel:one" }],
      windows: [{ id: "window:target", active_surface_id: null }],
    },
    target,
  },
};

describe("transfer payload strictness", () => {
  test("refuses any corrupted panel completion or abort", () => {
    expectStrict(assertValidPanelTransferResponse, committed);
    expectStrict(assertValidPanelTransferResponse, { status: "aborted", abort });
  });

  test("refuses any corrupted target binding or selector value", () => {
    for (const binding of bindings) {
      expectStrict(assertValidTransferTargetBinding, binding);
    }
    expectStrict(assertValidTransferCommitSelector, {
      kind: "explicit_zone",
      drop_zone_id: "zone:center",
    });
    expectStrict(assertValidTransferCommitSelector, {
      kind: "screen_point",
      point: { x: -20, y: 160 },
    });
  });

  test("refuses any corrupted session, lease, cancellation or client snapshot", () => {
    expectStrict(assertValidTransferSessionResponse, {
      status: "started",
      session: {
        protocol_version: 1,
        request_id: "request:start",
        payload: { protocol_version: 1, session_id },
      },
    });
    expectStrict(assertValidTransferLeaseResponse, {
      status: "published",
      lease: {
        protocol_version: 1,
        request_id: "request:lease",
        client_id: "client:main",
        client_epoch: 1,
        generation: 2,
        zone_count: 3,
      },
    });
    expectStrict(assertValidTransferCancelResponse, {
      status: "cancelled",
      cancellation: {
        protocol_version: 1,
        request_id: "request:cancel",
        session_id,
        status: "cancelled",
      },
    });
    expectStrict(assertValidTransferClientSnapshot, {
      protocol_version: 1,
      client_id: "client:main",
      client_epoch: 1,
    });
  });
});
