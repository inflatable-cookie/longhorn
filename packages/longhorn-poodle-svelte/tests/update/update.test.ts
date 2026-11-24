import { render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import { UPDATE_PROTOCOL_VERSION, UpdateController } from "@inflatable-cookie/longhorn/update";
import type {
  UpdateOutcomeProjection,
  UpdatePort,
  UpdateSnapshot,
} from "@inflatable-cookie/longhorn/update";

import ObserveOnlyHarness from "./ObserveOnlyHarness.svelte";
import UpdateHarness from "./UpdateHarness.svelte";
import {
  actionableVersion,
  deferUpdate,
  installUpdate,
} from "../../src/update/poodle/UpdateSurface.svelte.ts";

function snapshot(overrides: Partial<UpdateSnapshot> = {}): UpdateSnapshot {
  return {
    protocolVersion: UPDATE_PROTOCOL_VERSION,
    authorityEpoch: 1,
    channel: "production",
    installedVersion: "1.3.0",
    availability: { state: "upToDate" },
    deferral: null,
    staged: null,
    progress: { state: "idle" },
    ...overrides,
  };
}

const OFFER = snapshot({
  availability: { state: "offer", version: "1.4.0", reason: "staged", notes: null },
});

/** The digest a retained artifact carries. 64 lowercase hex digits. */
const DIGEST = "a".repeat(64);

/** A port whose snapshot changes between reads, and that can notify. */
class Port implements UpdatePort {
  state: UpdateSnapshot = snapshot();
  /** Every command the controller sent, in order. */
  readonly calls: Array<Record<string, unknown>> = [];
  #listeners: Array<(event: unknown) => void> = [];
  #progressListener: ((event: unknown) => void) | undefined;
  prepareWait: Promise<void> = Promise.resolve();
  async snapshot(): Promise<unknown> {
    return this.state;
  }
  async check(): Promise<unknown> {
    this.calls.push({ name: "check" });
    return this.#committed();
  }
  async selectChannel(): Promise<unknown> {
    this.calls.push({ name: "selectChannel" });
    return this.#committed();
  }
  async defer(command?: { version?: string; cause?: unknown }): Promise<unknown> {
    this.calls.push({ name: "defer", version: command?.version, cause: command?.cause });
    return this.#committed();
  }
  async prepare(command?: { version?: string }): Promise<unknown> {
    this.calls.push({ name: "prepare", version: command?.version });
    return this.#committed();
  }
  async apply(command?: { version?: string; digest?: string }): Promise<unknown> {
    this.calls.push({ name: "apply", version: command?.version, digest: command?.digest });
    return this.#committed();
  }
  async cancel(): Promise<unknown> {
    this.calls.push({ name: "cancel" });
    return this.#committed();
  }
  listen(listener: (event: unknown) => void) {
    this.#listeners.push(listener);
    return () => {};
  }
  async prepareWithProgress(command: { version: string }, listener: (event: unknown) => void) {
    this.#progressListener = listener;
    try {
      await this.prepareWait;
      return await this.prepare(command);
    } finally {
      this.#progressListener = undefined;
    }
  }
  notifyProgress(progress: UpdateSnapshot["progress"]): void {
    this.#progressListener?.({
      protocolVersion: UPDATE_PROTOCOL_VERSION,
      authorityEpoch: 1,
      progress,
    });
  }
  notify(): void {
    for (const listener of this.#listeners) {
      listener({ protocolVersion: UPDATE_PROTOCOL_VERSION, authorityEpoch: 1, kind: "checked" });
    }
  }
  #committed(): UpdateOutcomeProjection {
    return { status: "committed", snapshot: this.state };
  }
}

describe("update surface bindings", () => {
  it("offers a check before reporting that the install is up to date", async () => {
    const port = new Port();
    port.state = snapshot({ availability: { state: "notChecked" } });
    const controller = new UpdateController({ port });
    await controller.start();
    const mounted = render(UpdateHarness, { props: { controller } });

    expect(mounted.getByRole("button", { name: "Check for updates" })).toBeDefined();
    expect(mounted.container.textContent).not.toContain("up to date");

    port.state = snapshot();
    port.notify();
    await waitFor(() => expect(mounted.container.textContent).toContain("up to date"));
    mounted.unmount();
  });

  /**
   * The claim the binding exists for. `UpdateController` is a plain class, so
   * `controller.availability` is not a tracked dependency: without something
   * making the parent's expressions reactive, the authority can notify all it
   * likes and the DOM never moves.
   */
  it("re-renders when the authority notifies", async () => {
    const port = new Port();
    const controller = new UpdateController({ port });
    await controller.start();
    const mounted = render(UpdateHarness, { props: { controller } });

    await waitFor(() => expect(document.body.textContent).toContain("1.3.0"));

    port.state = OFFER;
    port.notify();

    await waitFor(() => expect(document.body.textContent).toContain("1.4.0"));
    mounted.unmount();
  });

  /**
   * `presence` is the whole point of the centre: no update, no icon, and no
   * space reserved for one either.
   */
  it("renders nothing at all until there is something to act on", async () => {
    const port = new Port();
    const controller = new UpdateController({ port });
    await controller.start();
    const mounted = render(UpdateHarness, { props: { controller, centre: true } });

    await waitFor(() => expect(mounted.container.textContent).toBe(""));
    expect(mounted.container.querySelector("button")).toBeNull();

    port.state = OFFER;
    port.notify();

    await waitFor(() => expect(mounted.container.querySelector("button")).not.toBeNull());
    mounted.unmount();
  });

  /**
   * Poodle's `observe` drives the view on its own, with props read once from a
   * plain class.
   *
   * Nothing here depends on that -- the sampler covers it -- but it is the
   * assumption behind ever deleting the sampler, and it was false for two
   * props until 2026-08-13. Asserted so their regression is visible here
   * rather than discovered the next time someone tries to simplify.
   */
  it("is driven by Poodle's observe prop alone", async () => {
    const port = new Port();
    const controller = new UpdateController({ port });
    await controller.start();
    const mounted = render(ObserveOnlyHarness, { props: { controller } });

    await waitFor(() => expect(document.body.textContent).toContain("1.3.0"));

    port.state = OFFER;
    port.notify();
    await controller.refresh();
    await new Promise((resolve) => {
      setTimeout(resolve, 50);
    });

    expect(document.body.textContent).toContain("1.4.0");
    mounted.unmount();
  });

  /**
   * The reason the adapter listens to the progress channel at all. The
   * authority does not hold its lock across the transfer, so the snapshot read
   * during a download still says what it said before it started; only the live
   * channel moves the bar.
   */
  it("exposes byte progress published out of band", async () => {
    const port = new Port();
    port.state = OFFER;
    const controller = new UpdateController({ port });
    await controller.start();
    const mounted = render(UpdateHarness, { props: { controller } });

    let finish: (() => void) | undefined;
    port.prepareWait = new Promise<void>((resolve) => {
      finish = resolve;
    });
    const pending = controller.prepare("1.4.0");
    try {
      port.notifyProgress({
        state: "downloading",
        received: 50,
        expected: 100,
        fraction: 0.5,
      });
      await waitFor(() =>
        expect(controller.progress).toEqual({
          state: "downloading",
          received: 50,
          expected: 100,
          fraction: 0.5,
        }),
      );
    } finally {
      finish?.();
      await pending;
      mounted.unmount();
    }
  });

  /**
   * One stage-apply decision for every binding.
   *
   * The update centre's popover and the settings status panel act on one
   * controller, so the command their install button sends is defined once,
   * beside the sampled projection, rather than twice inside the two
   * components. This asserts the shared decision: an offer is staged, a
   * retained artifact is applied and never staged again, and "not now" is the
   * operator's deferral rather than the gate's.
   */
  describe("shared commands", () => {
    it("stages an offer and applies a retained artifact", async () => {
      const port = new Port();
      port.state = OFFER;
      const controller = new UpdateController({ port });
      await controller.start();

      expect(actionableVersion(controller)).toBe("1.4.0");
      installUpdate(controller);
      await waitFor(() => expect(port.calls).toContainEqual({ name: "prepare", version: "1.4.0" }));
      expect(port.calls).not.toContainEqual(expect.objectContaining({ name: "apply" }));

      port.state = snapshot({
        availability: { state: "offer", version: "1.4.0", reason: "staged", notes: null },
        staged: { version: "1.4.0", channel: "production", digest: DIGEST },
        progress: { state: "readyToInstall", version: "1.4.0" },
      });
      port.notify();
      await controller.refresh();

      expect(actionableVersion(controller)).toBe("1.4.0");
      installUpdate(controller);
      await waitFor(() =>
        expect(port.calls).toContainEqual({ name: "apply", version: "1.4.0", digest: DIGEST }),
      );
      expect(port.calls.filter(({ name }) => name === "prepare")).toHaveLength(1);
    });

    it("defers the actionable version as the operator's own decision", async () => {
      const port = new Port();
      port.state = OFFER;
      const controller = new UpdateController({ port });
      await controller.start();

      deferUpdate(controller);
      await waitFor(() =>
        expect(port.calls).toContainEqual({
          name: "defer",
          version: "1.4.0",
          cause: { cause: "userPostponed" },
        }),
      );
    });

    it("does nothing when no version is actionable", async () => {
      const port = new Port();
      const controller = new UpdateController({ port });
      await controller.start();

      expect(actionableVersion(controller)).toBeUndefined();
      installUpdate(controller);
      deferUpdate(controller);
      await waitFor(() => expect(port.calls).toEqual([]));
    });
  });
});
