import { fireEvent, render, waitFor, within } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import { TauriTransport } from "@inflatable-cookie/longhorn-tauri";
import {
  createTauriOperationPort,
  OPERATION_CANCEL_COMMAND,
  OPERATION_SNAPSHOT_COMMAND,
} from "@inflatable-cookie/longhorn-tauri/operation";

import hostErrors from "../../../../fixtures/operation/host-errors.json";

import OperationPanelHarness from "./OperationPanelHarness.svelte";
import { OperationSession } from "../../src/operation/svelte.ts";
import {
  createMountedSession,
  loopholeSnapshot,
  MountedOperationPort,
  soundcheckSnapshot,
} from "./support.ts";

describe("operation Svelte and public Poodle adapters", () => {
  it("keeps windows independent and unmounts without cancelling host work", async () => {
    const first = createMountedSession();
    const second = createMountedSession();
    const firstMount = render(OperationPanelHarness, {
      props: { session: first.session, shape: "soundcheck" },
    });
    const secondMount = render(OperationPanelHarness, {
      props: { session: second.session, shape: "soundcheck" },
    });

    await waitFor(() => {
      expect(first.session.status.kind).toBe("ready");
      expect(second.session.status.kind).toBe("ready");
    });
    expect(first.port.listeners.size).toBe(1);
    expect(second.port.listeners.size).toBe(1);

    await fireEvent.click(
      within(firstMount.container).getByRole("button", { name: "Scan plug-ins" }),
    );
    expect(firstMount.getByText("soundcheck detail for operation:scan")).toBeTruthy();
    expect(second.session.selected).toBeUndefined();

    await Promise.resolve(firstMount.unmount());
    await waitFor(() => expect(first.port.listeners.size).toBe(0));
    expect(first.port.unlistenCount).toBe(1);
    expect(first.port.cancellations).toEqual([]);
    expect(second.port.listeners.size).toBe(1);
    await Promise.resolve(secondMount.unmount());
  });

  it("remounts from current host truth", async () => {
    const state = createMountedSession();
    const first = render(OperationPanelHarness, {
      props: { session: state.session, shape: "soundcheck" },
    });
    await waitFor(() => expect(state.session.status.kind).toBe("ready"));
    await Promise.resolve(first.unmount());
    state.port.snapshotValue = loopholeSnapshot();

    const second = render(OperationPanelHarness, {
      props: { session: state.session, shape: "loophole" },
    });
    expect(await second.findByRole("button", { name: "Render final sequence" })).toBeTruthy();
    expect(state.port.unlistenCount).toBe(1);
    await Promise.resolve(second.unmount());
    await waitFor(() => expect(state.port.unlistenCount).toBe(2));
  });

  it("lets two windows observe one authority without sharing selection", async () => {
    const port = new MountedOperationPort(soundcheckSnapshot());
    const firstSession = new OperationSession({ port });
    const secondSession = new OperationSession({ port });
    const first = render(OperationPanelHarness, {
      props: { session: firstSession, shape: "soundcheck" },
    });
    const second = render(OperationPanelHarness, {
      props: { session: secondSession, shape: "soundcheck" },
    });
    await waitFor(() => {
      expect(firstSession.status.kind).toBe("ready");
      expect(secondSession.status.kind).toBe("ready");
      expect(port.listeners.size).toBe(2);
    });

    await fireEvent.click(within(first.container).getByRole("button", { name: "Scan plug-ins" }));
    expect(firstSession.selected?.operationId).toBe("operation:scan");
    expect(secondSession.selected).toBeUndefined();

    port.publish(loopholeSnapshot());
    await waitFor(() => {
      expect(firstSession.active[0]?.operationId).toBe("operation:render-running");
      expect(secondSession.active[0]?.operationId).toBe("operation:render-running");
    });
    expect(firstSession.selected).toBeUndefined();

    await Promise.resolve(first.unmount());
    await Promise.resolve(second.unmount());
    await waitFor(() => expect(port.listeners.size).toBe(0));
    expect(port.unlistenCount).toBe(2);
  });

  it("shows a typed Tauri cancellation refusal and closes the dialog", async () => {
    const refusal = hostErrors[0];
    if (refusal === undefined) {
      throw new Error("missing host error sample");
    }
    const invoke = vi.fn(async (command: string, args: { query?: { requestId: string } }) => {
      if (command === OPERATION_SNAPSHOT_COMMAND) {
        return { requestId: args.query?.requestId, snapshot: loopholeSnapshot() };
      }
      if (command === OPERATION_CANCEL_COMMAND) {
        throw refusal;
      }
      throw new Error(`unexpected command ${command}`);
    });
    vi.stubGlobal("__TAURI_INTERNALS__", { invoke });
    const session = new OperationSession({
      port: createTauriOperationPort({
        transport: new TauriTransport(),
        nextRequestId: () => "request:test",
      }),
    });
    const renderPanel = render(OperationPanelHarness, {
      props: { session, shape: "loophole" },
    });

    try {
      await fireEvent.click(
        await renderPanel.findByRole("button", { name: "Cancel Render final sequence" }),
      );
      await fireEvent.click(renderPanel.getByRole("button", { name: "Request cancellation" }));
      await waitFor(() => {
        expect(renderPanel.getByText(refusal.message)).toBeTruthy();
        expect(renderPanel.queryByRole("alertdialog", { name: "Cancel operation?" })).toBeNull();
      });
    } finally {
      await Promise.resolve(renderPanel.unmount());
      vi.unstubAllGlobals();
    }
  });

  it("projects Soundcheck and Loophole through the same controlled panel", async () => {
    const soundcheck = createMountedSession(soundcheckSnapshot());
    const loophole = createMountedSession(loopholeSnapshot());
    const scanPanel = render(OperationPanelHarness, {
      props: { session: soundcheck.session, shape: "soundcheck" },
    });
    const renderPanel = render(OperationPanelHarness, {
      props: { session: loophole.session, shape: "loophole" },
    });
    await waitFor(() => {
      expect(soundcheck.session.status.kind).toBe("ready");
      expect(loophole.session.status.kind).toBe("ready");
    });

    expect(
      scanPanel
        .getByRole("progressbar", { name: "Scan plug-ins progress" })
        .getAttribute("aria-valuenow"),
    ).toBe("2");
    expect(
      renderPanel
        .getByRole("progressbar", { name: "Render final sequence progress" })
        .getAttribute("aria-valuenow"),
    ).toBe("0.65");
    expect(
      renderPanel
        .getByRole("progressbar", { name: "Render trailer progress" })
        .hasAttribute("aria-valuenow"),
    ).toBe(false);
    expect(renderPanel.getByText("Queued")).toBeTruthy();
    expect(renderPanel.getByText("Succeeded")).toBeTruthy();

    await fireEvent.click(
      renderPanel.getByRole("button", { name: "Cancel Render final sequence" }),
    );
    expect(renderPanel.getByRole("alertdialog", { name: "Cancel operation?" })).toBeTruthy();
    await fireEvent.click(renderPanel.getByRole("button", { name: "Request cancellation" }));
    await waitFor(() => {
      expect(loophole.port.cancellations).toHaveLength(1);
      expect(loophole.session.active[0]?.state).toBe("cancelling");
    });

    await fireEvent.click(
      renderPanel.getByRole("button", { name: "Dismiss Render opening titles" }),
    );
    await waitFor(() => {
      expect(loophole.port.mutations).toHaveLength(1);
      expect(loophole.session.recent).toHaveLength(0);
    });

    await Promise.resolve(scanPanel.unmount());
    await Promise.resolve(renderPanel.unmount());
  });
});
