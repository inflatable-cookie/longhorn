import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import {
  CROSS_WINDOW_DRAG_MIME_TYPE,
  CROSS_WINDOW_DRAG_PROTOCOL_VERSION,
} from "@inflatable-cookie/poodle-core";
import {
  LONGHORN_TRANSFER_MIME_TYPE,
  TRANSFER_CANCEL_COMMAND,
  TRANSFER_COMMIT_PANEL_COMMAND,
  TRANSFER_SNAPSHOT_COMMAND,
  TRANSFER_START_PANEL_COMMAND,
  TransferClient,
  type PanelTransferCommand,
  type TransferCancelRequest,
} from "@inflatable-cookie/longhorn/transfer";
import { TransferState } from "@inflatable-cookie/longhorn-poodle-svelte/transfer";

import fixture from "../../../../fixtures/transfer/protocol-v1.json";
import {
  createPanelTransferDragSource,
  createPanelTransferDropTarget,
} from "../../src/poodle/transfer.ts";
import LayoutDockHarness from "./LayoutDockHarness.svelte";
import {
  deferred,
  instance,
  loadShape,
  mountedBinding,
  resolvePanel,
  shapeDocument,
} from "./support.ts";

describe("public Poodle cross-window transfer seam", () => {
  it("arms before dragstart, writes only protocol payload, and commits an explicit lease zone", async () => {
    const sourceRequests: unknown[] = [];
    const commits: PanelTransferCommand[] = [];
    const sourceState = transferState("client:source", async (
      command,
      arguments_,
    ) => {
      if (command === TRANSFER_START_PANEL_COMMAND) {
        sourceRequests.push(arguments_.request);
        return fixture.session_responses[0];
      }
      if (command === TRANSFER_CANCEL_COMMAND) {
        return cancelled(arguments_.request as TransferCancelRequest);
      }
      throw new Error(`unexpected source command: ${command}`);
    });
    const targetState = transferState("client:target", async (
      command,
      arguments_,
    ) => {
      if (command === TRANSFER_COMMIT_PANEL_COMMAND) {
        commits.push(arguments_.request as PanelTransferCommand);
        return fixture.commit_responses[0];
      }
      throw new Error(`unexpected target command: ${command}`);
    });
    await Promise.all([sourceState.start(), targetState.start()]);

    let terminal = 0;
    const errors: unknown[] = [];
    const dragSource = createPanelTransferDragSource({
      state: sourceState,
      makeStartRequest: (panelInstanceId) => ({
        protocol_version: 1,
        request_id: "request:start-panel",
        panel_instance_id: panelInstanceId,
      }),
      reportError: (error) => errors.push(error),
    });
    const dropTarget = createPanelTransferDropTarget({
      state: targetState,
      selector: { kind: "explicit_zone", dropZoneId: "zone:center" },
      nextRequestId: () => "request:commit-zone",
      reportError: (error) => errors.push(error),
      onTerminal: () => {
        terminal += 1;
      },
    });
    const shape = loadShape("surface-bound");
    const sourceBinding = mountedBinding(
      shape.definitions,
      shapeDocument(shape, { primary: [instance("instance:a")] }),
      async () => new Promise(() => undefined),
    ).binding;
    const targetBinding = mountedBinding(
      shape.definitions,
      shapeDocument(shape, { secondary: [instance("instance:b")] }),
      async () => new Promise(() => undefined),
    ).binding;
    const source = render(LayoutDockHarness, {
      props: {
        binding: sourceBinding,
        resolvePanel,
        primaryCrossWindowDragSource: dragSource,
      },
    });
    const target = render(LayoutDockHarness, {
      props: {
        binding: targetBinding,
        resolvePanel,
      },
    });
    const sourceTab = source.getByRole("tab", { name: "A" });
    const dataTransfer = new DataTransfer();

    await fireEvent.pointerDown(sourceTab, { button: 0 });
    await waitFor(() =>
      expect(sourceState.preparation.status).toBe("prepared"),
    );
    expect(sourceTab.getAttribute("draggable")).toBe("true");
    await fireEvent.dragStart(sourceTab, { dataTransfer });
    expect(dataTransfer.getData(LONGHORN_TRANSFER_MIME_TYPE)).toBe("");
    const receipt = JSON.parse(
      dataTransfer.getData(CROSS_WINDOW_DRAG_MIME_TYPE),
    ) as { protocolVersion: number; token: string };
    expect(receipt).toEqual({
      protocolVersion: CROSS_WINDOW_DRAG_PROTOCOL_VERSION,
      token: "abababababababababababababababab",
    });
    expect(sourceRequests).toEqual([
      {
        protocol_version: 1,
        request_id: "request:start-panel",
        panel_instance_id: "instance:a",
      },
    ]);

    await dropTarget.commit(
      {
        receipt,
        subject: { kind: "poodle.dock-panel", id: "instance:a" },
        intent: {
          targetId: "secondary",
          position: "inside",
          operation: "move",
        },
      },
      new AbortController().signal,
    );
    await waitFor(() => expect(commits).toHaveLength(1));
    expect(commits[0]).toEqual({
      protocol_version: 1,
      request_id: "request:commit-zone",
      session_id: receipt.token,
      selector: {
        kind: "explicit_zone",
        drop_zone_id: "zone:center",
      },
    });
    await waitFor(() => expect(terminal).toBe(1));
    expect(errors).toEqual([]);
    await Promise.all([
      source.unmount(),
      target.unmount(),
      sourceState.destroy(),
      targetState.destroy(),
    ]);
  });

  it("rejects dragstart that races the prepared host session", async () => {
    const pending = deferred<unknown>();
    let cancellations = 0;
    const state = transferState("client:source", async (
      command,
      arguments_,
    ) => {
      if (command === TRANSFER_START_PANEL_COMMAND) return pending.promise;
      if (command === TRANSFER_CANCEL_COMMAND) {
        cancellations += 1;
        return cancelled(arguments_.request as TransferCancelRequest);
      }
      throw new Error(`unexpected command: ${command}`);
    });
    await state.start();
    const errors: unknown[] = [];
    const shape = loadShape("surface-bound");
    const binding = mountedBinding(
      shape.definitions,
      shapeDocument(shape, { primary: [instance("instance:a")] }),
      async () => new Promise(() => undefined),
    ).binding;
    const screen = render(LayoutDockHarness, {
      props: {
        binding,
        resolvePanel,
        primaryCrossWindowDragSource: createPanelTransferDragSource({
          state,
          makeStartRequest: (panelInstanceId) => ({
            protocol_version: 1,
            request_id: "request:start-panel",
            panel_instance_id: panelInstanceId,
          }),
          reportError: (error) => errors.push(error),
        }),
      },
    });
    const sourceTab = screen.getByRole("tab", { name: "A" });
    const dataTransfer = new DataTransfer();

    await fireEvent.pointerDown(sourceTab, { button: 0 });
    await fireEvent.dragStart(sourceTab, { dataTransfer });
    expect(dataTransfer.getData(CROSS_WINDOW_DRAG_MIME_TYPE)).toBe("");
    expect(sourceTab.getAttribute("draggable")).toBe("false");
    await fireEvent.pointerUp(sourceTab, { button: 0 });

    pending.resolve(fixture.session_responses[0]);
    await waitFor(() => expect(cancellations).toBe(1));
    expect(state.preparation.status).toBe("idle");
    expect(errors).toEqual([]);
    await screen.unmount();
    await state.destroy();
  });

  it("cancels an armed session when its public Poodle region unmounts", async () => {
    let cancellations = 0;
    const state = transferState("client:source", async (
      command,
      arguments_,
    ) => {
      if (command === TRANSFER_START_PANEL_COMMAND) {
        return fixture.session_responses[0];
      }
      if (command === TRANSFER_CANCEL_COMMAND) {
        cancellations += 1;
        return cancelled(arguments_.request as TransferCancelRequest);
      }
      throw new Error(`unexpected command: ${command}`);
    });
    await state.start();
    const shape = loadShape("surface-bound");
    const binding = mountedBinding(
      shape.definitions,
      shapeDocument(shape, { primary: [instance("instance:a")] }),
      async () => new Promise(() => undefined),
    ).binding;
    const screen = render(LayoutDockHarness, {
      props: {
        binding,
        resolvePanel,
        primaryCrossWindowDragSource: createPanelTransferDragSource({
          state,
          makeStartRequest: (panelInstanceId) => ({
            protocol_version: 1,
            request_id: "request:start-panel",
            panel_instance_id: panelInstanceId,
          }),
          reportError: () => undefined,
        }),
      },
    });

    await fireEvent.pointerDown(
      screen.getByRole("tab", { name: "A" }),
      { button: 0 },
    );
    await waitFor(() => expect(state.preparation.status).toBe("prepared"));
    await screen.unmount();
    await waitFor(() => {
      expect(cancellations).toBe(1);
      expect(state.preparation.status).toBe("idle");
    });
    await state.destroy();
  });
});

function transferState(
  clientId: string,
  invoke: MockTransport["invoke"],
): TransferState {
  const transport: MockTransport = {
    async listen() {
      return () => undefined;
    },
    async invoke(command, arguments_) {
      if (command === TRANSFER_SNAPSHOT_COMMAND) {
        return {
          protocol_version: 1,
          client_id: clientId,
          client_epoch: 1,
          current_lease_generation: null,
        };
      }
      return invoke(command, arguments_);
    },
  };
  return new TransferState({
    client: new TransferClient(transport),
    makeCancellationRequest: (sessionId) => ({
      protocol_version: 1,
      request_id: "request:cancel",
      session_id: sessionId,
    }),
  });
}

interface MockTransport {
  listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<() => void>;
  invoke(
    command: string,
    arguments_: Record<string, unknown>,
  ): Promise<unknown>;
}

function cancelled(request: TransferCancelRequest) {
  return {
    status: "cancelled",
    cancellation: {
      protocol_version: 1,
      request_id: request.request_id,
      session_id: request.session_id,
      status: "cancelled",
    },
  };
}
