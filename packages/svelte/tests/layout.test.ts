import { describe, expect, it } from "vitest";

import type {
  LayoutDocument,
  LayoutMutationRejection,
  LayoutMutationRequest,
  LayoutMutationReceipt,
} from "@inflatable-cookie/longhorn-layout";

import {
  LayoutState,
  type LayoutDispatchResult,
} from "../src/layout.svelte.ts";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolve_) => {
    resolve = resolve_;
  });
  return { promise, resolve };
}

const document = (revision: number): LayoutDocument => ({
  revision,
  containers: [],
  panel_instances: [],
});

const request: LayoutMutationRequest = {
  request_id: "layout:request-1",
  expected_revision: 1,
  command: {
    kind: "set_region_collapsed",
    container_id: "container:main",
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
        container_id: "container:main",
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
      panel_instances: [
        { id: "optimistic", definition_id: "definition:test" },
      ],
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

  it("reports unsupported dispatch without treating it as transport failure", async () => {
    const state = new LayoutState({
      unsupportedReason: "consumer supplied no layout endpoint",
    });
    await state.start();
    state.accept(document(1));

    await expect(
      state.dispatch(request, (current) => current),
    ).rejects.toThrow("capability is unsupported");
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
      panel_instances: [
        { id: "optimistic", definition_id: "definition:test" },
      ],
    }));

    expect(result.status).toBe("rejected");
    expect(state.authoritative?.revision).toBe(2);
    expect(state.projected?.panel_instances).toEqual([]);
    expect(state.pendingRequestIds).toEqual([]);
  });
});
