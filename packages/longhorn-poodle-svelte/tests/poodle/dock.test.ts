import { fireEvent, render } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

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

describe("LayoutDockRegion", () => {
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
    const target = screen.getByRole("tab", { name: "B" });
    const dataTransfer = new DataTransfer();

    await fireEvent.dragStart(source, { dataTransfer });
    await fireEvent.dragOver(target, { dataTransfer });
    await fireEvent.drop(target, { dataTransfer });

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
    const dataTransfer = new DataTransfer();

    await fireEvent.dragStart(screen.getByRole("tab", { name: "A" }), {
      dataTransfer,
    });
    const target = screen.getByRole("region", { name: "Secondary dock" });
    await fireEvent.dragOver(target, { dataTransfer });
    await fireEvent.drop(target, { dataTransfer });

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
