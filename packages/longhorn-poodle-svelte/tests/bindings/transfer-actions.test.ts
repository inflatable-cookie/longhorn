import { waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { EventTransport } from "@inflatable-cookie/longhorn/core";
import {
  TRANSFER_CLIENT_CHANGED_EVENT,
  TRANSFER_PUBLISH_LEASE_COMMAND,
  TRANSFER_SNAPSHOT_COMMAND,
  TransferClient,
  type TransferClientSnapshot,
  type TransferLeaseRequest,
  type TransferLeaseResponse,
} from "@inflatable-cookie/longhorn/transfer";

import {
  DropZoneLeaseRegistry,
  TransferState,
} from "../../src/bindings/transfer.svelte.ts";

describe("DropZoneLeaseRegistry", () => {
  it("measures local geometry and publishes complete replacement leases", async () => {
    const requests: TransferLeaseRequest[] = [];
    let listener: ((payload: unknown) => void) | undefined;
    const snapshot: TransferClientSnapshot = {
      protocol_version: 1,
      client_id: "client:target",
      client_epoch: 4,
      current_lease_generation: null,
    };
    const transport: EventTransport = {
      async listen(event, receive) {
        expect(event).toBe(TRANSFER_CLIENT_CHANGED_EVENT);
        listener = receive;
        return () => undefined;
      },
      async invoke(command, arguments_) {
        if (command === TRANSFER_SNAPSHOT_COMMAND) return snapshot;
        if (command === TRANSFER_PUBLISH_LEASE_COMMAND) {
          const request = arguments_.request as TransferLeaseRequest;
          requests.push(request);
          return published(request);
        }
        throw new Error(`unexpected command: ${command}`);
      },
    };
    const state = new TransferState({
      client: new TransferClient(transport),
      makeCancellationRequest: (sessionId) => ({
        protocol_version: 1,
        request_id: "cancel",
        session_id: sessionId,
      }),
      makeLeaseReleaseRequest: (current, lease) => ({
        protocol_version: 1,
        request_id: "lease:release",
        client_id: current.client_id,
        client_epoch: current.client_epoch,
        generation: lease.generation + 1,
        zones: [],
      }),
    });
    await state.start();

    const errors: unknown[] = [];
    const registry = new DropZoneLeaseRegistry({
      state,
      nextRequestId: (() => {
        let value = 0;
        return () => `lease:${++value}`;
      })(),
      reportError: (error) => errors.push(error),
      observeGeometry: () => () => undefined,
    });
    const first = document.createElement("section");
    const second = document.createElement("section");
    first.getBoundingClientRect = () =>
      rectangle(20.25, 30.5, 400, 260);
    second.getBoundingClientRect = () =>
      rectangle(440, 30, 300, 260);
    const firstAction = registry.zone(first, {
      id: "zone:center",
      insertion_position: 1,
      accepted_capability: "move_panel",
      target: {
        kind: "panel_region",
        host_binding_id: "binding:target",
        document_id: "app.layout",
        revision: 7,
        surface_id: "container:target",
        region_id: "center",
      },
    });
    const secondAction = registry.zone(second, {
      id: "zone:surface",
      insertion_position: null,
      accepted_capability: "move_surface",
      target: {
        kind: "surface_window",
        host_binding_id: "binding:surface",
        document_id: "app.surfaces",
        revision: 9,
      },
    });

    await registry.start();
    expect(requests[0]).toMatchObject({
      client_id: "client:target",
      client_epoch: 4,
      generation: 1,
    });
    expect(requests[0].zones).toEqual([
      {
        id: "zone:center",
        bounds: {
          origin: { x: 20.25, y: 30.5 },
          size: { width: 400, height: 260 },
        },
        insertion_position: 1,
        accepted_capability: "move_panel",
        target: expect.objectContaining({ region_id: "center" }),
      },
      expect.objectContaining({ id: "zone:surface" }),
    ]);

    listener?.({
      ...snapshot,
      client_id: "client:replacement",
      client_epoch: 5,
    });
    firstAction.destroy();
    await waitFor(() =>
      expect(requests.at(-1)).toMatchObject({
        client_id: "client:replacement",
        client_epoch: 5,
        zones: [expect.objectContaining({ id: "zone:surface" })],
      }),
    );

    secondAction.destroy();
    await registry.destroy();
    expect(requests.at(-1)).toMatchObject({
      client_id: "client:replacement",
      client_epoch: 5,
      zones: [],
    });
    expect(errors).toEqual([]);
    await state.destroy();
  });
});

function published(request: TransferLeaseRequest): TransferLeaseResponse {
  return {
    status: "published",
    lease: {
      protocol_version: 1,
      request_id: request.request_id,
      client_id: request.client_id,
      client_epoch: request.client_epoch,
      generation: request.generation,
      zone_count: request.zones.length,
    },
  };
}

function rectangle(
  x: number,
  y: number,
  width: number,
  height: number,
): DOMRect {
  return {
    x,
    y,
    left: x,
    top: y,
    width,
    height,
    right: x + width,
    bottom: y + height,
    toJSON: () => ({}),
  };
}
