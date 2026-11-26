import { describe, expect, it } from "vitest";

import type {
  LayoutMutationRejection,
  LayoutMutationRequest,
  LayoutMutationReceipt,
} from "@inflatable-cookie/longhorn/layout";
import type { SurfaceDocument } from "@inflatable-cookie/longhorn/surfaces";

import { LayoutState } from "../../src/bindings/layout.svelte.ts";
import type { LayoutDispatchResult } from "../../src/bindings/layout.svelte.ts";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((resolve_, reject_) => {
    resolve = resolve_;
    reject = reject_;
  });
  return { promise, resolve, reject };
}

const document = (revision: number): SurfaceDocument => ({
  revision,
  surfaces: [],
  panel_instances: [],
  windows: [],
});

const request: LayoutMutationRequest = {
  request_id: "layout:request-1",
  expected_revision: 1,
  command: {
    kind: "set_region_collapsed",
    surface_id: "surface:main",
    region_id: "left",
    collapsed: true,
  },
};

function committed(revision: number): LayoutDispatchResult {
  return {
    status: "committed",
    receipt: {
      request_id: request.request_id,
      previous_revision: revision - 1,
      committed_revision: revision,
      outcome: {
        kind: "region_collapsed_set",
        surface_id: "surface:main",
        region_id: "left",
        previous_collapsed: false,
        committed_collapsed: true,
      },
      authoritative_document: document(revision),
    } satisfies LayoutMutationReceipt,
  };
}

describe("LayoutState", () => {
  it("keeps layout consumer-fed and rejects stale optimistic completion", async () => {
    const result = deferred<LayoutDispatchResult>();
    const state = new LayoutState({ dispatch: () => result.promise });
    await state.start();
    state.accept(document(1));

    const pending = state.dispatch(request, (current) => ({
      ...current,
      panel_instances: [{ id: "optimistic", definition_id: "definition:test" }],
    }));
    expect(state.pendingRequestIds).toEqual([request.request_id]);
    expect(state.projected?.panel_instances).toHaveLength(1);

    state.accept(document(3));
    result.resolve(committed(2));
    await pending;

    expect(state.authoritative?.revision).toBe(3);
    expect(state.projected?.panel_instances).toEqual([]);
    expect(state.pendingRequestIds).toEqual([]);
  });

  it("keeps a stopped projection empty when dispatch commits late", async () => {
    const result = deferred<LayoutDispatchResult>();
    const state = new LayoutState({ dispatch: () => result.promise });
    await state.start();
    state.accept(document(1));
    const pending = state.dispatch(request, (current) => current);

    await state.stop();
    result.resolve(committed(2));
    await expect(pending).resolves.toEqual(committed(2));

    expect(state.status.kind).toBe("idle");
    expect(state.authoritative).toBeUndefined();
    expect(state.projected).toBeUndefined();
    expect(state.pendingRequestIds).toEqual([]);
  });

  it("does not let a late dispatch failure cancel a reused request after restart", async () => {
    const old = deferred<LayoutDispatchResult>();
    const next = deferred<LayoutDispatchResult>();
    let dispatch = old;
    const state = new LayoutState({ dispatch: () => dispatch.promise });
    await state.start();
    state.accept(document(1));
    const first = state.dispatch(request, (current) => current);

    await state.stop();
    await state.start();
    state.accept(document(1));
    dispatch = next;
    const second = state.dispatch(request, (current) => current);
    old.reject(new Error("late dispatch failure"));

    await expect(first).rejects.toThrow("late dispatch failure");
    expect(state.pendingRequestIds).toEqual([request.request_id]);
    expect(state.status.kind).toBe("ready");
    next.resolve(committed(2));
    await second;
  });

  it("reports unsupported dispatch without treating it as transport failure", async () => {
    const state = new LayoutState({
      unsupportedReason: "consumer supplied no layout endpoint",
    });
    await state.start();
    expect(state.accept(document(1))).toBe(true);
    expect(state.status.kind).toBe("unsupported");
    expect(state.projected?.revision).toBe(1);
    state.reconnecting();
    expect(state.status.kind).toBe("unsupported");

    await expect(state.dispatch(request, (current) => current)).rejects.toThrow(
      "capability is unsupported",
    );
    expect(state.status).toEqual({
      kind: "unsupported",
      reason: "consumer supplied no layout endpoint",
    });
  });

  it("reconciles rejection authority and clears only its request", async () => {
    const rejection: LayoutDispatchResult = {
      status: "rejected",
      rejection: {
        request_id: request.request_id,
        current_revision: 2,
        code: "stale_revision",
        detail: "newer authority",
        authoritative_document: document(2),
      } satisfies LayoutMutationRejection,
    };
    const state = new LayoutState({
      dispatch: async () => rejection,
    });
    await state.start();
    state.accept(document(1));

    const result = await state.dispatch(request, (current) => ({
      ...current,
      panel_instances: [{ id: "optimistic", definition_id: "definition:test" }],
    }));

    expect(result.status).toBe("rejected");
    expect(state.authoritative?.revision).toBe(2);
    expect(state.projected?.panel_instances).toEqual([]);
    expect(state.pendingRequestIds).toEqual([]);
  });
});
