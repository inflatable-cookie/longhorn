import { describe, expect, it } from "bun:test";
import {
  assertForkDeleteResult,
  assertForkPruneResult,
} from "../../src/history-tree/validation.ts";
import { isRecord } from "../../src/core/record.ts";
import { ForkHistoryClient } from "../../src/history-tree/client.ts";
import { SerializedForkHistoryPort } from "../../src/history-tree/serialized.ts";
import type { ForkHistoryPort } from "../../src/history-tree/ports.ts";
import {
  branchPage,
  changed,
  clone,
  committed,
  continuationPage,
  forkHistoryGolden,
  forkHistoryFixture,
  pathPage,
  removal,
  snapshot,
} from "./support.ts";

function port(): ForkHistoryPort {
  return {
    snapshot: async () => clone(snapshot),
    path: async () => clone(pathPage),
    branches: async () => clone(branchPage),
    continuations: async () => clone(continuationPage),
    deleteContinuation: async () => ({ status: "deleted", receipt: clone(removal) }),
    prune: async () => ({ status: "unchanged" }) as const,
    navigate: async () => clone(committed),
    listen: (listener) => {
      listener(clone(changed));
      return () => {};
    },
    nextPlanId: () => "plan:test",
  };
}
describe("fork-history direct and serialized conformance", () => {
  for (const [name, value] of [
    ["direct", port()],
    ["serialized", new SerializedForkHistoryPort(port())],
  ] as const) {
    it(name, async () => {
      const client = new ForkHistoryClient(value);
      expect(await client.snapshot()).toEqual(snapshot);
      expect(
        await client.path({
          protocolVersion: 2,
          authorityEpoch: 7,
          historyId: "history:tree",
          expectedRevision: 4,
          target: { kind: "default" },
          offset: 0,
          limit: 2,
        }),
      ).toEqual(pathPage);
      expect(
        await client.branches({
          protocolVersion: 2,
          authorityEpoch: 7,
          historyId: "history:tree",
          expectedRevision: 4,
          offset: 0,
          limit: 2,
        }),
      ).toEqual(branchPage);
    });
  }
  it("accepts typed retention refusals on both operations", async () => {
    const value = port();
    const rejection = { code: "staleRevision", detail: "stale", refreshRequired: true } as const;
    value.deleteContinuation = async () => ({ status: "rejected", rejection });
    value.prune = async () => ({
      status: "rejected",
      rejection: { ...rejection, code: "zeroLimit" },
    });
    const client = new ForkHistoryClient(value);
    const common = {
      protocolVersion: 2 as const,
      authorityEpoch: 7,
      historyId: "history:tree",
      expectedRevision: 4,
    };
    expect(await client.deleteContinuation({ ...common, entryId: "entry:a" })).toEqual({
      status: "rejected",
      rejection,
    });
    expect(
      (await client.prune({ ...common, maximumEntries: 0, maximumEncodedWeight: 1 })).status,
    ).toBe("rejected");
  });
  it("decodes Rust-generated retention refusals", async () => {
    const golden = forkHistoryFixture().retentionRejections;
    if (!isRecord(golden)) {
      throw new Error("invalid retention examples");
    }
    const value = port();
    const client = new ForkHistoryClient(value);
    const common = {
      protocolVersion: 2 as const,
      authorityEpoch: 7,
      historyId: "history:tree",
      expectedRevision: 4,
    };
    for (const key of ["deletePinned", "deleteCurrent"]) {
      const expected = golden[key];
      assertForkDeleteResult(expected);
      value.deleteContinuation = async () => expected;
      expect(await client.deleteContinuation({ ...common, entryId: "entry:a" })).toEqual(expected);
    }
    for (const key of ["pruneStale", "pruneZero"]) {
      const expected = golden[key];
      assertForkPruneResult(expected);
      value.prune = async () => expected;
      expect(await client.prune({ ...common, maximumEntries: 0, maximumEncodedWeight: 1 })).toEqual(
        expected,
      );
    }
  });
  it("rejects future protocols and payload fields", async () => {
    const future = port();
    future.snapshot = async () => ({ ...snapshot, protocolVersion: 3 });
    await expect(new ForkHistoryClient(future).snapshot()).rejects.toThrow("exact protocol");
    const payload = port();
    payload.snapshot = async () => ({ ...snapshot, payload: {} });
    await expect(new ForkHistoryClient(payload).snapshot()).rejects.toThrow("payload");
  });
  it("decodes the Rust-generated snapshot golden", async () => {
    const golden = forkHistoryGolden();
    const value = port();
    value.snapshot = async () => golden;
    expect(await new ForkHistoryClient(value).snapshot()).toEqual(golden);
  });
  it("rejects page requests above the generated hard ceiling", async () => {
    await expect(
      new ForkHistoryClient(port()).path({
        protocolVersion: 2,
        authorityEpoch: 7,
        historyId: "history:tree",
        expectedRevision: 4,
        target: { kind: "default" },
        offset: 0,
        limit: 257,
      }),
    ).rejects.toThrow("maximum is 256");
  });
});
