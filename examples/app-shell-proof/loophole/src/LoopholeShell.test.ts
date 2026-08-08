import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { EventTransport } from "@inflatable-cookie/longhorn/core";
import { SurfaceTransferClient } from "@inflatable-cookie/longhorn/surface-transfer";
import {
  SURFACE_SNAPSHOT_COMMAND,
  SurfaceClient,
} from "@inflatable-cookie/longhorn/surfaces";
import {
  TRANSFER_CANCEL_COMMAND,
  TRANSFER_COMMIT_PANEL_COMMAND,
  TRANSFER_SNAPSHOT_COMMAND,
  TRANSFER_START_PANEL_COMMAND,
  TransferClient,
  type TransferCancelRequest,
} from "@inflatable-cookie/longhorn/transfer";
import { LayoutState } from "@inflatable-cookie/longhorn-poodle-svelte/layout";
import { SurfaceState } from "@inflatable-cookie/longhorn-poodle-svelte/surfaces";
import { SurfaceTransferState } from "@inflatable-cookie/longhorn-poodle-svelte/surface-transfer";
import { TransferState } from "@inflatable-cookie/longhorn-poodle-svelte/transfer";

import LoopholeShell from "./LoopholeShell.svelte";
import {
  layoutDocument,
  surfaceSnapshot,
} from "./model.ts";

function states(onDispose: () => void, onCancel: () => void) {
  const transport: EventTransport = {
    async listen() {
      return onDispose;
    },
    async invoke(command, arguments_) {
      if (command === SURFACE_SNAPSHOT_COMMAND) return surfaceSnapshot;
      if (command === TRANSFER_SNAPSHOT_COMMAND) {
        return {
          protocol_version: 1,
          client_id: "client:studio",
          client_epoch: 2,
          current_lease_generation: null,
        };
      }
      if (command === TRANSFER_START_PANEL_COMMAND) {
        const request = arguments_.request as {
          request_id: string;
        };
        return {
          status: "started",
          session: {
            protocol_version: 1,
            request_id: request.request_id,
            payload: {
              protocol_version: 1,
              session_id: "abababababababababababababababab",
            },
          },
        };
      }
      if (command === TRANSFER_COMMIT_PANEL_COMMAND) {
        const request = arguments_.request as {
          request_id: string;
        };
        return {
          status: "aborted",
          abort: {
            protocol_version: 1,
            request_id: request.request_id,
            source: { domain: "transfer", code: "no_target" },
            message: "proof target rejects after authoritative recheck",
            retryable: true,
            session_consumed: true,
          },
        };
      }
      if (command === TRANSFER_CANCEL_COMMAND) {
        onCancel();
        const request = arguments_.request as TransferCancelRequest;
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
      throw new Error(`unexpected command: ${command}`);
    },
  };
  const transferState = new TransferState({
    client: new TransferClient(transport),
    makeCancellationRequest: (sessionId) => ({
      protocol_version: 1,
      request_id: "request:cancel-shell",
      session_id: sessionId,
    }),
  });
  return {
    layoutState: new LayoutState({
      dispatch: async () =>
        new Promise(() => {
          // No layout mutation in the shell proof.
        }),
    }),
    surfaceState: new SurfaceState({
      client: new SurfaceClient(transport),
    }),
    transferState,
    surfaceTransferState: new SurfaceTransferState({
      client: new SurfaceTransferClient(transport),
      cancelSession: async () => undefined,
    }),
  };
}

describe("Loophole shell", () => {
  it("mounts the full hierarchy and public cross-window drag seam", async () => {
    let disposals = 0;
    let cancellations = 0;
    const shellStates = states(
      () => {
        disposals += 1;
      },
      () => {
        cancellations += 1;
      },
    );
    const events: string[] = [];
    const screen = render(LoopholeShell, {
      props: {
        ...shellStates,
        async loadLayoutAuthority() {
          events.push("authority");
          return layoutDocument;
        },
        async reveal() {
          events.push("reveal");
        },
      },
    });

    await waitFor(() =>
      expect(screen.getByRole("main", { name: "Loophole studio" })).toBeTruthy(),
    );
    await waitFor(() => expect(events).toEqual(["authority", "reveal"]));
    expect(screen.container.querySelector("[data-display] [data-window] [data-surface] [data-layout-container]")).toBeTruthy();
    expect(screen.getAllByRole("region")).toHaveLength(8);
    expect(document.documentElement.dataset.theme).toBe("midnight");

    const sourceTab = screen.getByRole("tab", { name: "Mixer" });
    await fireEvent.pointerDown(sourceTab, { button: 0 });
    await waitFor(() =>
      expect(shellStates.transferState.preparation.status).toBe("prepared"),
    );
    const preparation = shellStates.transferState.preparation;
    if (preparation.status !== "prepared") {
      throw new Error("panel session was not prepared");
    }
    expect(preparation.response.session.payload).toEqual({
      protocol_version: 1,
      session_id: "abababababababababababababababab",
    });

    await shellStates.transferState.commitPanel({
      protocol_version: 1,
      request_id: "request:drop-proof",
      session_id: preparation.response.session.payload.session_id,
      selector: {
        kind: "explicit_zone",
        drop_zone_id: "zone:secondary",
      },
    });
    expect(shellStates.transferState.completion.status).toBe("aborted");
    expect(cancellations).toBe(0);

    await screen.unmount();
    await waitFor(() => expect(disposals).toBe(2));
  });

  it("cancels an armed host session when the full shell is destroyed", async () => {
    let cancellations = 0;
    const shellStates = states(
      () => undefined,
      () => {
        cancellations += 1;
      },
    );
    const screen = render(LoopholeShell, {
      props: {
        ...shellStates,
        loadLayoutAuthority: async () => layoutDocument,
        reveal: async () => undefined,
      },
    });
    await waitFor(() => expect(screen.getByRole("tab", { name: "Mixer" })).toBeTruthy());
    await fireEvent.pointerDown(
      screen.getByRole("tab", { name: "Mixer" }),
      { button: 0 },
    );
    await waitFor(() =>
      expect(shellStates.transferState.preparation.status).toBe("prepared"),
    );

    await screen.unmount();
    await waitFor(() => expect(cancellations).toBe(1));
  });
});
