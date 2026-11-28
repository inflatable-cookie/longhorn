import { render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { EventTransport } from "@inflatable-cookie/longhorn/core";
import {
  SURFACE_CHANGED_EVENT,
  SURFACE_MUTATE_COMMAND,
  SURFACE_SNAPSHOT_COMMAND,
  SurfaceClient,
} from "@inflatable-cookie/longhorn/surfaces";
import type {
  SurfaceChangedEvent,
  SurfaceMutationRequest,
  SurfaceMutationResponse,
  SurfaceSnapshot,
} from "@inflatable-cookie/longhorn/surfaces";

import fixture from "../../../../fixtures/surfaces/protocol-v1.json";
import { SurfaceState } from "../../src/bindings/surfaces.svelte.ts";
import LifecycleHarness from "./LifecycleHarness.svelte";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

// The client rejects a reply that does not echo its request id.
function committed(requestId: string): SurfaceMutationResponse {
  const response = fixture.responses[0] as Extract<
    SurfaceMutationResponse,
    { status: "committed" }
  >;
  return { ...response, receipt: { ...response.receipt, request_id: requestId } };
}

describe("SurfaceState", () => {
  it("reconciles request-keyed optimism against a newer snapshot", async () => {
    let snapshot = fixture.snapshots[0] as SurfaceSnapshot;
    const mutation = deferred<SurfaceMutationResponse>();
    let listener: ((payload: unknown) => void) | undefined;
    const transport: EventTransport = {
      async listen(event, receive) {
        expect(event).toBe(SURFACE_CHANGED_EVENT);
        listener = receive;
        return () => undefined;
      },
      async invoke(command) {
        if (command === SURFACE_SNAPSHOT_COMMAND) {
          return snapshot;
        }
        if (command === SURFACE_MUTATE_COMMAND) {
          return mutation.promise;
        }
        throw new Error(`unexpected command: ${command}`);
      },
    };
    const state = new SurfaceState({
      client: new SurfaceClient(transport),
    });
    await state.start();

    const request = fixture.commands[0] as SurfaceMutationRequest;
    const pending = state.mutate(request, (document) => ({
      ...document,
      surfaces: [
        ...document.surfaces,
        {
          id: "surface:optimistic",
          schema_id: "schema:optimistic",
          label: null,
          presentation: { kind: "regional" },
          regions: [],
          sizing_slots: [],
          host_preferences: [],
        },
      ],
    }));
    expect(state.pendingRequestIds).toEqual([request.request_id]);

    const nextSnapshot = fixture.snapshots[1];
    if (nextSnapshot === undefined) {
      throw new Error("missing surface fixture");
    }
    snapshot = {
      ...nextSnapshot,
      epoch: 8,
      revision: 1,
      document: { ...nextSnapshot.document, revision: 1 },
    } as SurfaceSnapshot;
    listener?.({
      protocol_version: 1,
      epoch: snapshot.epoch,
      revision: snapshot.revision,
    } satisfies SurfaceChangedEvent);
    await Promise.resolve();
    await Promise.resolve();

    mutation.resolve(fixture.responses[0] as SurfaceMutationResponse);
    await pending;
    expect(state.snapshot?.epoch).toBe(8);
    expect(state.snapshot?.revision).toBe(1);
    expect(state.pendingRequestIds).toEqual([]);
    expect(state.operations).toEqual([
      {
        requestId: request.request_id,
        status: "committed",
        response: fixture.responses[0],
      },
    ]);
    expect(state.projectedDocument?.surfaces.some(({ id }) => id === "surface:optimistic")).toBe(
      false,
    );
    await state.destroy();
  });

  it("bounds settled operation feedback and keeps pending requests until stop", async () => {
    const delayed = deferred<SurfaceMutationResponse>();
    const state = new SurfaceState({
      client: new SurfaceClient({
        async listen() {
          return () => undefined;
        },
        async invoke(command, arguments_) {
          if (command === SURFACE_SNAPSHOT_COMMAND) {
            return fixture.snapshots[0];
          }
          const request = arguments_.request as SurfaceMutationRequest;
          return request.request_id === "request:delayed"
            ? delayed.promise
            : committed(request.request_id);
        },
      }),
    });
    await state.start();
    const request = fixture.commands[0] as SurfaceMutationRequest;
    const pending = state.mutate({ ...request, request_id: "request:delayed" }, (doc) => doc);
    for (let index = 0; index < 35; index++) {
      await state.mutate({ ...request, request_id: `request:recent-${index}` }, (doc) => doc);
    }
    expect(state.operations.map(({ requestId }) => requestId)).toEqual([
      "request:delayed",
      ...Array.from({ length: 32 }, (_, index) => `request:recent-${index + 3}`),
    ]);
    expect(state.pendingRequestIds).toEqual(["request:delayed"]);

    await state.stop();
    delayed.resolve(committed("request:delayed"));
    await pending;
    expect(state.operations).toEqual([]);
    expect(state.status.kind).toBe("idle");
  });

  it("does not leak late listener registration after unmount", async () => {
    const registration = deferred<() => void>();
    let snapshots = 0;
    let unlistens = 0;
    const transport: EventTransport = {
      listen() {
        return registration.promise;
      },
      async invoke() {
        snapshots += 1;
        return fixture.snapshots[0];
      },
    };
    const state = new SurfaceState({
      client: new SurfaceClient(transport),
    });
    const mounted = render(LifecycleHarness, { props: { state } });
    await Promise.resolve();
    await Promise.resolve(mounted.unmount());

    registration.resolve(() => {
      unlistens += 1;
    });
    await state.stop();

    expect(snapshots).toBe(0);
    expect(unlistens).toBe(1);
    expect(state.status.kind).toBe("idle");
  });
});
