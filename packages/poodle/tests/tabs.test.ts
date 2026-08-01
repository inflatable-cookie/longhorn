import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { LayoutMutationRequest } from "@longhorn/layout";
import type { LayoutDispatchResult } from "@longhorn/svelte/layout";

import LayoutTabsHarness from "./LayoutTabsHarness.svelte";
import {
  deferred,
  instance,
  loadShape,
  mountedBinding,
  rejected,
  resolvePanel,
  shapeDocument,
} from "./support.ts";

describe("LayoutTabs", () => {
  it("projects selection immediately and restores rejection authority", async () => {
    const shape = loadShape("nucleus");
    const document = shapeDocument(shape, {
      left: [instance("instance:a"), instance("instance:b")],
    });
    const requests: LayoutMutationRequest[] = [];
    const pending = deferred<LayoutDispatchResult>();
    const { binding, errors } = mountedBinding(
      shape.definitions,
      document,
      (request) => {
        requests.push(request);
        return pending.promise;
      },
    );
    const screen = render(LayoutTabsHarness, {
      props: {
        binding,
        containerId: "container:primary",
        regionId: "left",
        resolvePanel,
      },
    });

    expect(screen.getByRole("tabpanel").textContent).toContain("A body");
    await fireEvent.click(screen.getByRole("tab", { name: "B" }));

    expect(requests).toEqual([
      {
        request_id: "request:poodle-1",
        expected_revision: 1,
        command: {
          kind: "activate_panel",
          panel_instance_id: "instance:b",
        },
      },
    ]);
    await waitFor(() =>
      expect(
        screen.getByRole("tab", { name: "B" }).getAttribute("aria-selected"),
      ).toBe("true"),
    );

    pending.resolve(rejected(requests[0], document));
    await waitFor(() =>
      expect(
        screen.getByRole("tab", { name: "A" }).getAttribute("aria-selected"),
      ).toBe("true"),
    );
    expect(errors).toEqual([]);
  });

  it("projects close with the authoritative active-panel fallback", async () => {
    const shape = loadShape("nucleus");
    const document = shapeDocument(shape, {
      left: [instance("instance:a"), instance("instance:b")],
    });
    const requests: LayoutMutationRequest[] = [];
    const pending = deferred<LayoutDispatchResult>();
    const { binding } = mountedBinding(
      shape.definitions,
      document,
      (request) => {
        requests.push(request);
        return pending.promise;
      },
    );
    const screen = render(LayoutTabsHarness, {
      props: {
        binding,
        containerId: "container:primary",
        regionId: "left",
        resolvePanel,
      },
    });

    await fireEvent.click(screen.getByRole("button", { name: "Close A" }));

    expect(requests[0].command).toEqual({
      kind: "close_panel",
      panel_instance_id: "instance:a",
    });
    await waitFor(() => {
      expect(screen.queryByRole("tab", { name: "A" })).toBeNull();
      expect(
        screen.getByRole("tab", { name: "B" }).getAttribute("aria-selected"),
      ).toBe("true");
    });
  });
});
