import assert from "node:assert/strict";
import { waitFor } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import type { LayoutMutationRequest } from "@inflatable-cookie/longhorn/layout";
import type { LayoutDispatchResult } from "@inflatable-cookie/longhorn-poodle-svelte/layout";

import {
  MissingPanelPresentationError,
  createPoodleLayoutBinding,
} from "../../src/poodle/index.ts";
import type { LayoutMutationState } from "../../src/poodle/index.ts";
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
    const shape = loadShape("window-bound");
    const document = shapeDocument(shape, {
      left: [instance("instance:a")],
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

    expect(() => binding.region("surface:primary", "left", () => null)).toThrow(
      MissingPanelPresentationError,
    );
  });

  it("rejects a non-collapsible split mapping before rendering", () => {
    const shape = loadShape("window-bound");
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

    expect(() => binding.collapsibleRegionState("surface:primary", "left")).toThrow(
      "region does not support collapse: left",
    );
  });

  it("transiently reveals only compatible hidden regions without mutation", () => {
    const shape = loadShape("surface-bound");
    const document = shapeDocument(shape, {
      primary: [instance("instance:a")],
    });
    const before = JSON.stringify(document);
    const { binding } = mountedBinding(shape.definitions, document, async () => {
      throw new Error("projection must not dispatch");
    });

    const ordinary = binding.regionVisibilities("surface:primary");
    const moving = binding.regionVisibilities("surface:primary", "instance:a");

    expect(ordinary.map(({ region_id, state }) => [region_id, state])).toEqual([
      ["navigation", "hidden"],
      ["activity", "hidden"],
      ["primary", "visible"],
      ["secondary", "hidden"],
      ["inspector", "hidden"],
      ["timeline", "hidden"],
      ["console", "hidden"],
      ["status", "visible"],
    ]);
    expect(moving.filter(({ state }) => state === "transiently_revealed")).toEqual([
      { region_id: "secondary", state: "transiently_revealed" },
    ]);
    expect(JSON.stringify(binding.document)).toBe(before);
  });

  it("serializes mutations against the latest authoritative revision", async () => {
    const requests: Array<LayoutMutationRequest> = [];
    const pendings: Array<ReturnType<typeof deferred<LayoutDispatchResult>>> = [];
    const shape = loadShape("window-bound");
    const document = shapeDocument(shape, {
      center_bottom: [instance("instance:a"), instance("instance:b")],
    });
    const { binding, errors, state } = mountedBinding(shape.definitions, document, (request) => {
      requests.push(request);
      const pending = deferred<LayoutDispatchResult>();
      pendings.push(pending);
      return pending.promise;
    });

    binding.activate("instance:b");
    binding.setCollapsed("surface:primary", "center_bottom", true);

    await waitFor(() => expect(requests).toHaveLength(1));
    const pending0 = pendings[0];
    assert(pending0);
    const request0 = requests[0];
    assert(request0);
    pending0.resolve(rejected(request0, { ...document, revision: 2 }));

    await waitFor(() => expect(requests).toHaveLength(2));
    expect(requests.map(({ expected_revision }) => expected_revision)).toEqual([1, 2]);
    expect(requests[1]?.command).toEqual({
      kind: "set_region_collapsed",
      surface_id: "surface:primary",
      region_id: "center_bottom",
      collapsed: true,
    });

    const pending1 = pendings[1];
    assert(pending1);
    const request1 = requests[1];
    assert(request1);
    pending1.resolve(rejected(request1, { ...document, revision: 3 }));
    await waitFor(() => expect(state.projected?.revision).toBe(3));
    expect(errors).toEqual([]);
  });
});
