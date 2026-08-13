import { render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import {
  UPDATE_PROTOCOL_VERSION,
  UpdateController,
  type UpdateOutcomeProjection,
  type UpdatePort,
  type UpdateSnapshot,
} from "@inflatable-cookie/longhorn/update";

import ObserveOnlyHarness from "./ObserveOnlyHarness.svelte";
import UpdateHarness from "./UpdateHarness.svelte";

function snapshot(overrides: Partial<UpdateSnapshot> = {}): UpdateSnapshot {
  return {
    protocolVersion: UPDATE_PROTOCOL_VERSION,
    authorityEpoch: 1,
    channel: "production",
    installedVersion: "1.3.0",
    availability: { state: "upToDate" },
    deferral: null,
    progress: { state: "idle" },
    ...overrides,
  };
}

const OFFER = snapshot({
  availability: { state: "offer", version: "1.4.0", reason: "staged", notes: null },
});

/** A port whose snapshot changes between reads, and that can notify. */
class Port implements UpdatePort {
  state: UpdateSnapshot = snapshot();
  #listeners: ((event: unknown) => void)[] = [];
  async snapshot(): Promise<unknown> { return this.state; }
  async check(): Promise<unknown> { return this.#committed(); }
  async selectChannel(): Promise<unknown> { return this.#committed(); }
  async defer(): Promise<unknown> { return this.#committed(); }
  async install(): Promise<unknown> { return this.#committed(); }
  listen(listener: (event: unknown) => void) {
    this.#listeners.push(listener);
    return () => {};
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
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(document.body.textContent).toContain("1.4.0");
    mounted.unmount();
  });
});
