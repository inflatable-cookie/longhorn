import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { LayoutMutationRequest } from "@longhorn/layout";
import type { LayoutDispatchResult } from "@longhorn/svelte/layout";

import LayoutSplitHarness from "./LayoutSplitHarness.svelte";
import {
  deferred,
  loadShape,
  mountedBinding,
  rejected,
  shapeDocument,
} from "./support.ts";

describe("LayoutSplitView", () => {
  it("projects an empty hidden pane without dispatching durable collapse", () => {
    const shape = loadShape("nucleus");
    const { binding } = mountedBinding(
      shape.definitions,
      shapeDocument(shape, {}),
      async () => {
        throw new Error("hidden projection must not dispatch");
      },
    );
    const screen = render(LayoutSplitHarness, {
      props: { binding, primaryHidden: true },
    });

    expect(
      screen.getByLabelText("Nucleus workspace split").dataset.primaryCollapsed,
    ).toBe("true");
    expect(screen.getByText("Navigation").style.flexBasis).toBe("0px");
    expect(screen.getByRole("separator").tabIndex).toBe(-1);
    expect(
      screen.queryByRole("button", { name: "Expand primary" }),
    ).toBeNull();
  });

  it("binds collapse and sizing to authoritative commands", async () => {
    const requests: LayoutMutationRequest[] = [];
    const pendings: Array<ReturnType<typeof deferred<LayoutDispatchResult>>> =
      [];
    const shape = loadShape("nucleus");
    const document = shapeDocument(shape, {});
    const { binding, errors } = mountedBinding(
      shape.definitions,
      document,
      (request) => {
        requests.push(request);
        const pending = deferred<LayoutDispatchResult>();
        pendings.push(pending);
        return pending.promise;
      },
    );
    const screen = render(LayoutSplitHarness, {
      props: { binding },
    });

    await fireEvent.click(
      screen.getByRole("button", { name: "Collapse primary" }),
    );
    expect(requests[0].command).toEqual({
      kind: "set_region_collapsed",
      container_id: "container:primary",
      region_id: "center_bottom",
      collapsed: true,
    });
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Expand primary" }),
      ).toBeTruthy(),
    );

    pendings[0].resolve(rejected(requests[0], document));
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Collapse primary" }),
      ).toBeTruthy(),
    );

    const split = screen.getByLabelText("Nucleus workspace split");
    split.getBoundingClientRect = () =>
      ({
        x: 0,
        y: 0,
        left: 0,
        top: 0,
        right: 1000,
        bottom: 500,
        width: 1000,
        height: 500,
        toJSON: () => ({}),
      }) as DOMRect;
    await fireEvent.keyDown(screen.getByRole("separator"), {
      key: "ArrowRight",
    });

    expect(requests[1].expected_revision).toBe(1);
    expect(requests[1].command.kind).toBe("set_sizing_slot");
    if (requests[1].command.kind === "set_sizing_slot") {
      expect(requests[1].command.ratio).toBeGreaterThan(250_000);
    }
    expect(errors).toEqual([]);
  });
});
