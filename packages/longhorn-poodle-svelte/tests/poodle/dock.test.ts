import { fireEvent, render } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  LayoutMutationRequest,
} from "@inflatable-cookie/longhorn/layout";
import type { LayoutDispatchResult } from "@inflatable-cookie/longhorn-poodle-svelte/layout";

import LayoutDockHarness from "./LayoutDockHarness.svelte";
import {
  deferred,
  instance,
  loadShape,
  mountedBinding,
  resolvePanel,
  shapeDocument,
} from "./support.ts";

function box(
  element: HTMLElement,
  x: number,
  y: number,
  width: number,
  height: number,
): void {
  const rect = {
    x,
    y,
    width,
    height,
    top: y,
    left: x,
    right: x + width,
    bottom: y + height,
    toJSON() {
      return this;
    },
  } as DOMRect;
  element.getBoundingClientRect = () => rect;
  element.setPointerCapture = vi.fn();
  element.releasePointerCapture = vi.fn();
  element.hasPointerCapture = () => false;
}

function layoutTabs(container: HTMLElement): void {
  [...container.querySelectorAll<HTMLElement>("section")].forEach(
    (region, regionIndex) => {
      const originX = regionIndex * 400;
      box(region, originX, 0, 400, 100);
      [...region.querySelectorAll<HTMLElement>(".poodle-tabs__item")].forEach(
        (item, index) => {
          box(item, originX + index * 100, 0, 100, 30);
          const tab = item.querySelector<HTMLElement>(".poodle-tabs__tab");
          if (tab) box(tab, originX + index * 100, 0, 100, 30);
        },
      );
    },
  );
}

describe("LayoutDockRegion", () => {
  beforeEach(() => {
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      callback(0);
      return 1;
    });
    vi.stubGlobal("cancelAnimationFrame", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("accepts showTabs=false without dispatching", () => {
    // Strip omission is proven in Poodle's DockRegion tests. Here we only
    // assert the LayoutDockRegion forward stays mount-safe against the
    // published peer until that peer version lands in this graph.
    const shape = loadShape("surface-bound");
    const document = shapeDocument(shape, {
      primary: [instance("instance:a")],
    });
    const { binding } = mountedBinding(
      shape.definitions,
      document,
      async () => {
        throw new Error("tabless dock must not dispatch");
      },
    );
    const screen = render(LayoutDockHarness, {
      props: { binding, resolvePanel, showTabs: false },
    });

    expect(screen.getByText("A primary body")).toBeTruthy();
  });

  it("keeps static panel rendering consumer-owned through a snippet", () => {
    const shape = loadShape("surface-bound");
    const document = shapeDocument(shape, {
      primary: [instance("instance:a")],
    });
    const { binding } = mountedBinding(
      shape.definitions,
      document,
      async () =>
        new Promise<LayoutDispatchResult>(() => {
          // No interaction in the snippet proof.
        }),
    );
    const screen = render(LayoutDockHarness, {
      props: { binding, resolvePanel, staticPrimary: true },
    });

    expect(screen.getByText("A static panel")).toBeTruthy();
  });

  it("dispatches a complete same-region reorder", async () => {
    const requests: LayoutMutationRequest[] = [];
    const pending = deferred<LayoutDispatchResult>();
    const shape = loadShape("surface-bound");
    const document = shapeDocument(shape, {
      primary: [instance("instance:a"), instance("instance:b")],
    });
    const { binding } = mountedBinding(
      shape.definitions,
      document,
      (request) => {
        requests.push(request);
        return pending.promise;
      },
    );
    const screen = render(LayoutDockHarness, {
      props: { binding, resolvePanel },
    });
    const source = screen.getByRole("tab", { name: "A" });
    source.focus();
    await fireEvent.keyDown(source, { key: "ArrowRight", altKey: true });

    expect(requests[0]).toEqual({
      request_id: "request:poodle-1",
      expected_revision: 1,
      command: {
        kind: "reorder_region",
        surface_id: "surface:primary",
        region_id: "primary",
        panel_instance_ids: ["instance:b", "instance:a"],
      },
    });
  });

  it("uses Poodle's public panel-drop callback for an eligible move", async () => {
    const requests: LayoutMutationRequest[] = [];
    const pending = deferred<LayoutDispatchResult>();
    const shape = loadShape("surface-bound");
    const document = shapeDocument(shape, {
      primary: [instance("instance:a")],
      secondary: [instance("instance:b")],
    });
    const { binding } = mountedBinding(
      shape.definitions,
      document,
      (request) => {
        requests.push(request);
        return pending.promise;
      },
    );
    const screen = render(LayoutDockHarness, {
      props: { binding, resolvePanel },
    });
    layoutTabs(screen.container);

    const source = screen.getByRole("tab", { name: "A" });
    await fireEvent.pointerDown(source, {
      button: 0,
      buttons: 1,
      pointerId: 1,
      pointerType: "mouse",
      isPrimary: true,
      clientX: 50,
      clientY: 15,
    });
    await fireEvent.pointerMove(source, {
      buttons: 1,
      pointerId: 1,
      pointerType: "mouse",
      isPrimary: true,
      clientX: 90,
      clientY: 15,
    });
    await fireEvent.pointerMove(source, {
      buttons: 1,
      pointerId: 1,
      pointerType: "mouse",
      isPrimary: true,
      clientX: 420,
      clientY: 15,
    });
    await fireEvent.pointerUp(source, {
      button: 0,
      buttons: 0,
      pointerId: 1,
      pointerType: "mouse",
      isPrimary: true,
      clientX: 420,
      clientY: 15,
    });

    expect(requests[0]).toEqual({
      request_id: "request:poodle-1",
      expected_revision: 1,
      command: {
        kind: "move_panel",
        panel_instance_id: "instance:a",
        target_surface_id: "surface:primary",
        target_region_id: "secondary",
        insertion_index: 1,
      },
    });
  });
});
