import { render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { EventTransport } from "@inflatable-cookie/longhorn/core";
import {
  SURFACE_CHANGED_EVENT,
  SURFACE_MUTATE_COMMAND,
  SURFACE_SNAPSHOT_COMMAND,
  SurfaceClient,
  type SurfaceChangedEvent,
  type SurfaceMutationRequest,
  type SurfaceMutationResponse,
  type SurfaceSnapshot,
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
          layout_container_id: "container:optimistic",
          label: null,
          presentation: { kind: "regional" },
          host_preferences: [],
        },
      ],
    }));
    expect(state.pendingRequestIds).toEqual([request.request_id]);

    snapshot = {
      ...fixture.snapshots[1],
      epoch: 8,
      revision: 1,
      document: { ...fixture.snapshots[1].document, revision: 1 },
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
    expect(
      state.projectedDocument?.surfaces.some(
        ({ id }) => id === "surface:optimistic",
      ),
    ).toBe(false);
    await state.destroy();
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
    await mounted.unmount();

    registration.resolve(() => {
      unlistens += 1;
    });
    await state.stop();

    expect(snapshots).toBe(0);
    expect(unlistens).toBe(1);
    expect(state.status.kind).toBe("idle");
  });
});
