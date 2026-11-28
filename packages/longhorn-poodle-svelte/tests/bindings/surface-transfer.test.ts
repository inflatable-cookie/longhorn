import { fireEvent, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { InvokeTransport } from "@inflatable-cookie/longhorn/core";
import { LONGHORN_TRANSFER_MIME_TYPE } from "@inflatable-cookie/longhorn/transfer";
import {
  TRANSFER_COMMIT_SURFACE_COMMAND,
  TRANSFER_START_SURFACE_COMMAND,
  SurfaceTransferClient,
} from "@inflatable-cookie/longhorn/surface-transfer";
import type {
  SurfaceTransferCommand,
  SurfaceSessionResponse,
  SurfaceSessionStartRequest,
} from "@inflatable-cookie/longhorn/surface-transfer";

import fixture from "../../../../fixtures/surface-transfer/protocol-v1.json";
import {
  SurfaceTransferState,
  surfaceTransferDrag,
  surfaceTransferDrop,
} from "../../src/bindings/surface-transfer.svelte.ts";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

describe("SurfaceTransferState", () => {
  it.each([TRANSFER_START_SURFACE_COMMAND, TRANSFER_COMMIT_SURFACE_COMMAND])(
    "keeps Surface transfer available after %s rejects",
    async (failedCommand) => {
      const failure = new Error("command refused");
      let shouldFail = true;
      const state = new SurfaceTransferState({
        client: new SurfaceTransferClient({
          async invoke(command) {
            if (command === failedCommand && shouldFail) {
              shouldFail = false;
              throw failure;
            }
            return command === TRANSFER_START_SURFACE_COMMAND
              ? fixture.session_responses[0]
              : fixture.commit_responses[0];
          },
        }),
        async cancelSession() {},
      });
      const start = fixture.session_requests[0] as SurfaceSessionStartRequest;
      const commit = fixture.commit_requests[0] as SurfaceTransferCommand;

      await state.start();
      if (failedCommand === TRANSFER_START_SURFACE_COMMAND) {
        await expect(state.prepare(start)).rejects.toBe(failure);
        expect(state.preparation).toEqual({ status: "failed", error: failure });
      } else {
        await state.prepare(start);
        await expect(state.commit(commit)).rejects.toBe(failure);
        expect(state.completion).toEqual({ status: "failed", error: failure });
      }
      expect(state.status.kind).toBe("ready");
      await state.cancelPreparation();
      await expect(state.prepare(start)).resolves.toMatchObject({ status: "started" });
      await state.destroy();
    },
  );

  it("cancels a preparation that completes after teardown", async () => {
    const preparation = deferred<SurfaceSessionResponse>();
    const cancelled: Array<string> = [];
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

    const prepare = state.prepare(fixture.session_requests[0] as SurfaceSessionStartRequest);
    const stop = state.stop();
    preparation.resolve(fixture.session_responses[0] as SurfaceSessionResponse);
    await Promise.all([prepare, stop]);

    expect(cancelled).toEqual(["cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"]);
    expect(state.status.kind).toBe("idle");
    expect(state.preparation.status).toBe("idle");
  });

  it("resets local state and remains destroyed when cancelling fails", async () => {
    const failure = new Error("cancel failed");
    const state = new SurfaceTransferState({
      client: new SurfaceTransferClient({
        async invoke() {
          return fixture.session_responses[0];
        },
      }),
      async cancelSession() {
        throw failure;
      },
    });
    await state.start();
    await state.prepare(fixture.session_requests[0] as SurfaceSessionStartRequest);

    await expect(state.destroy()).rejects.toBe(failure);
    expect(state.status.kind).toBe("idle");
    expect(state.preparation.status).toBe("idle");
    await state.stop();
    await expect(state.start()).rejects.toThrow(/destroyed/);
  });

  it("arms a Surface synchronously and commits a checked screen point", async () => {
    const commands: Array<SurfaceTransferCommand> = [];
    const errors: Array<unknown> = [];
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
    const dragOptions = {
      state,
      makeStartRequest: () => fixture.session_requests[0] as SurfaceSessionStartRequest,
      reportError: (error: unknown) => errors.push(error),
    };
    const dragAction = surfaceTransferDrag(source, dragOptions);
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
    dragAction.update({ ...dragOptions, reportError: (error) => errors.push(error) });
    source.dispatchEvent(dragEvent("dragstart", dataTransfer));
    dragAction.update({ ...dragOptions, reportError: (error) => errors.push(error) });
    const protectedStore = new DataTransfer();
    protectedStore.setData(LONGHORN_TRANSFER_MIME_TYPE, "");
    const dragOver = dragEvent("dragover", protectedStore);
    target.dispatchEvent(dragOver);
    expect(dragOver.defaultPrevented).toBe(true);
    expect(protectedStore.dropEffect).toBe("move");
    const unrelatedDragOver = dragEvent("dragover", new DataTransfer());
    target.dispatchEvent(unrelatedDragOver);
    expect(unrelatedDragOver.defaultPrevented).toBe(false);
    target.dispatchEvent(dragEvent("drop", dataTransfer, { screenX: 800, screenY: 420 }));

    await waitFor(() => expect(commands.length + errors.length).toBeGreaterThan(0));
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
