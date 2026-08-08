import { fireEvent, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { InvokeTransport } from "@inflatable-cookie/longhorn-core";
import {
  TRANSFER_COMMIT_SURFACE_COMMAND,
  TRANSFER_START_SURFACE_COMMAND,
  SurfaceTransferClient,
  type SurfaceTransferCommand,
  type SurfaceSessionResponse,
  type SurfaceSessionStartRequest,
} from "@inflatable-cookie/longhorn-surface-transfer";

import fixture from "../../../fixtures/surface-transfer/protocol-v1.json";
import {
  SurfaceTransferState,
  surfaceTransferDrag,
  surfaceTransferDrop,
} from "../src/surface-transfer.svelte.ts";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

describe("SurfaceTransferState", () => {
  it("cancels a preparation that completes after teardown", async () => {
    const preparation = deferred<SurfaceSessionResponse>();
    const cancelled: string[] = [];
    const transport: InvokeTransport = {
      async invoke(command) {
        if (command === TRANSFER_START_SURFACE_COMMAND) {
          return preparation.promise;
        }
        throw new Error(`unexpected command: ${command}`);
      },
    };
    const state = new SurfaceTransferState({
      client: new SurfaceTransferClient(transport),
      async cancelSession(sessionId) {
        cancelled.push(sessionId);
      },
    });
    await state.start();

    const prepare = state.prepare(
      fixture.session_requests[0] as SurfaceSessionStartRequest,
    );
    const stop = state.stop();
    preparation.resolve(
      fixture.session_responses[0] as SurfaceSessionResponse,
    );
    await Promise.all([prepare, stop]);

    expect(cancelled).toEqual([
      "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
    ]);
    expect(state.status.kind).toBe("idle");
    expect(state.preparation.status).toBe("idle");
  });

  it("arms a Surface synchronously and commits a checked screen point", async () => {
    const commands: SurfaceTransferCommand[] = [];
    const errors: unknown[] = [];
    const transport: InvokeTransport = {
      async invoke(command, arguments_) {
        if (command === TRANSFER_START_SURFACE_COMMAND) {
          return fixture.session_responses[0];
        }
        if (command === TRANSFER_COMMIT_SURFACE_COMMAND) {
          commands.push(arguments_.request as SurfaceTransferCommand);
          return fixture.commit_responses[0];
        }
        throw new Error(`unexpected command: ${command}`);
      },
    };
    const state = new SurfaceTransferState({
      client: new SurfaceTransferClient(transport),
      async cancelSession() {
        throw new Error("committed session must not be cancelled");
      },
    });
    await state.start();
    const source = document.createElement("div");
    const target = document.createElement("div");
    document.body.append(source, target);
    const dragAction = surfaceTransferDrag(source, {
      state,
      makeStartRequest: () =>
        fixture.session_requests[0] as SurfaceSessionStartRequest,
      reportError: (error) => errors.push(error),
    });
    let terminal = 0;
    const dropAction = surfaceTransferDrop(target, {
      state,
      selector: { kind: "screen_point" },
      nextRequestId: () => "request:commit-provisioned",
      reportError: (error) => errors.push(error),
      onTerminal: () => {
        terminal += 1;
      },
    });
    const dataTransfer = new DataTransfer();

    await fireEvent.pointerDown(source, { button: 0 });
    await waitFor(() => expect(state.preparation.status).toBe("prepared"));
    source.dispatchEvent(dragEvent("dragstart", dataTransfer));
    target.dispatchEvent(dragEvent("dragover", dataTransfer));
    target.dispatchEvent(
      dragEvent("drop", dataTransfer, { screenX: 800, screenY: 420 }),
    );

    await waitFor(() =>
      expect(commands.length + errors.length).toBeGreaterThan(0),
    );
    expect(errors).toEqual([]);
    expect(commands).toHaveLength(1);
    expect(commands[0]).toEqual({
      protocol_version: 1,
      request_id: "request:commit-provisioned",
      session_id: "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
      selector: {
        kind: "screen_point",
        point: { x: 800, y: 420 },
      },
    });
    await waitFor(() => expect(terminal).toBe(1));
    source.dispatchEvent(dragEvent("dragend", dataTransfer));
    expect(errors).toEqual([]);

    dragAction.destroy();
    dropAction.destroy();
    await state.destroy();
  });
});

function dragEvent(
  type: string,
  dataTransfer: DataTransfer,
  coordinates?: { readonly screenX: number; readonly screenY: number },
): DragEvent {
  const event = new DragEvent(type, {
    bubbles: true,
    cancelable: true,
  });
  Object.defineProperty(event, "dataTransfer", { value: dataTransfer });
  if (coordinates) {
    Object.defineProperties(event, {
      screenX: { value: coordinates.screenX },
      screenY: { value: coordinates.screenY },
    });
  }
  return event;
}
