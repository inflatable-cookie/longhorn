import { waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { LayoutMutationRequest } from "@longhorn/layout";
import type { LayoutDispatchResult } from "@longhorn/svelte/layout";

import {
  MissingPanelPresentationError,
  createPoodleLayoutBinding,
  type LayoutMutationState,
} from "../src/index.ts";
import {
  deferred,
  instance,
  loadShape,
  mountedBinding,
  rejected,
  shapeDocument,
} from "./support.ts";

describe("PoodleLayoutBinding", () => {
  it("makes missing consumer presentation explicit", () => {
    const shape = loadShape("nucleus");
    const document = shapeDocument(shape, {
      main: [instance("instance:a")],
    });
    const state: LayoutMutationState = {
      projected: document,
      dispatch: async () => {
        throw new Error("not reached");
      },
    };
    const binding = createPoodleLayoutBinding({
      state,
      definitions: shape.definitions,
      nextRequestId: () => "request:unused",
      onError: () => undefined,
    });

    expect(() =>
      binding.region("container:primary", "main", () => null),
    ).toThrow(MissingPanelPresentationError);
  });

  it("rejects a non-collapsible split mapping before rendering", () => {
    const shape = loadShape("nucleus");
    const state: LayoutMutationState = {
      projected: shapeDocument(shape, {}),
      dispatch: async () => {
        throw new Error("not reached");
      },
    };
    const binding = createPoodleLayoutBinding({
      state,
      definitions: shape.definitions,
      nextRequestId: () => "request:unused",
      onError: () => undefined,
    });

    expect(() =>
      binding.collapsibleRegionState("container:primary", "main"),
    ).toThrow("region does not support collapse: main");
  });

  it("transiently reveals only compatible hidden regions without mutation", () => {
    const shape = loadShape("loophole");
    const document = shapeDocument(shape, {
      primary: [instance("instance:a")],
    });
    const before = JSON.stringify(document);
    const { binding } = mountedBinding(
      shape.definitions,
      document,
      async () => {
        throw new Error("projection must not dispatch");
      },
    );

    const ordinary = binding.regionVisibilities("container:primary");
    const moving = binding.regionVisibilities(
      "container:primary",
      "instance:a",
    );

    expect(
      ordinary.map(({ region_id, state }) => [region_id, state]),
    ).toEqual([
      ["navigation", "hidden"],
      ["activity", "hidden"],
      ["primary", "visible"],
      ["secondary", "hidden"],
      ["inspector", "hidden"],
      ["timeline", "hidden"],
      ["console", "hidden"],
      ["status", "visible"],
    ]);
    expect(
      moving.filter(({ state }) => state === "transiently_revealed"),
    ).toEqual([
      { region_id: "secondary", state: "transiently_revealed" },
    ]);
    expect(JSON.stringify(binding.document)).toBe(before);
  });

  it("serializes mutations against the latest authoritative revision", async () => {
    const requests: LayoutMutationRequest[] = [];
    const pendings: Array<ReturnType<typeof deferred<LayoutDispatchResult>>> =
      [];
    const shape = loadShape("nucleus");
    const document = shapeDocument(shape, {
      navigation: [instance("instance:a"), instance("instance:b")],
    });
    const { binding, errors, state } = mountedBinding(
      shape.definitions,
      document,
      (request) => {
        requests.push(request);
        const pending = deferred<LayoutDispatchResult>();
        pendings.push(pending);
        return pending.promise;
      },
    );

    binding.activate("instance:b");
    binding.setCollapsed("container:primary", "navigation", true);

    await waitFor(() => expect(requests).toHaveLength(1));
    pendings[0].resolve(
      rejected(requests[0], { ...document, revision: 2 }),
    );

    await waitFor(() => expect(requests).toHaveLength(2));
    expect(requests.map(({ expected_revision }) => expected_revision)).toEqual([
      1, 2,
    ]);
    expect(requests[1].command).toEqual({
      kind: "set_region_collapsed",
      container_id: "container:primary",
      region_id: "navigation",
      collapsed: true,
    });

    pendings[1].resolve(
      rejected(requests[1], { ...document, revision: 3 }),
    );
    await waitFor(() => expect(state.projected?.revision).toBe(3));
    expect(errors).toEqual([]);
  });
});
