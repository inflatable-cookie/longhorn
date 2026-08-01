import { fireEvent, render, waitFor, within } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import OperationPanelHarness from "./OperationPanelHarness.svelte";
import { OperationSession } from "../src/svelte.ts";
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

    await firstMount.unmount();
    await waitFor(() => expect(first.port.listeners.size).toBe(0));
    expect(first.port.unlistenCount).toBe(1);
    expect(first.port.cancellations).toEqual([]);
    expect(second.port.listeners.size).toBe(1);
    await secondMount.unmount();
  });

  it("remounts from current host truth", async () => {
    const state = createMountedSession();
    const first = render(OperationPanelHarness, {
      props: { session: state.session, shape: "soundcheck" },
    });
    await waitFor(() => expect(state.session.status.kind).toBe("ready"));
    await first.unmount();
    state.port.snapshotValue = loopholeSnapshot();

    const second = render(OperationPanelHarness, {
      props: { session: state.session, shape: "loophole" },
    });
    expect(await second.findByRole("button", { name: "Render final sequence" })).toBeTruthy();
    expect(state.port.unlistenCount).toBe(1);
    await second.unmount();
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

    await fireEvent.click(
      within(first.container).getByRole("button", { name: "Scan plug-ins" }),
    );
    expect(firstSession.selected?.operationId).toBe("operation:scan");
    expect(secondSession.selected).toBeUndefined();

    port.publish(loopholeSnapshot());
    await waitFor(() => {
      expect(firstSession.active[0]?.operationId).toBe("operation:render-running");
      expect(secondSession.active[0]?.operationId).toBe("operation:render-running");
    });
    expect(firstSession.selected).toBeUndefined();

    await first.unmount();
    await second.unmount();
    await waitFor(() => expect(port.listeners.size).toBe(0));
    expect(port.unlistenCount).toBe(2);
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

    expect(scanPanel.getByRole("progressbar", { name: "Scan plug-ins progress" }).getAttribute("aria-valuenow")).toBe("2");
    expect(renderPanel.getByRole("progressbar", { name: "Render final sequence progress" }).getAttribute("aria-valuenow")).toBe("0.65");
    expect(renderPanel.getByRole("progressbar", { name: "Render trailer progress" }).hasAttribute("aria-valuenow")).toBe(false);
    expect(renderPanel.getByText("Queued")).toBeTruthy();
    expect(renderPanel.getByText("Succeeded")).toBeTruthy();

    await fireEvent.click(renderPanel.getByRole("button", { name: "Cancel Render final sequence" }));
    expect(renderPanel.getByRole("alertdialog", { name: "Cancel operation?" })).toBeTruthy();
    await fireEvent.click(renderPanel.getByRole("button", { name: "Request cancellation" }));
    await waitFor(() => {
      expect(loophole.port.cancellations).toHaveLength(1);
      expect(loophole.session.active[0]?.state).toBe("cancelling");
    });

    await fireEvent.click(renderPanel.getByRole("button", { name: "Dismiss Render opening titles" }));
    await waitFor(() => {
      expect(loophole.port.mutations).toHaveLength(1);
      expect(loophole.session.recent).toHaveLength(0);
    });

    await scanPanel.unmount();
    await renderPanel.unmount();
  });
});
