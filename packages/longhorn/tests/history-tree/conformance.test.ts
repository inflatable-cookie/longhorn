import { describe, expect, it } from "bun:test";
import { ForkHistoryClient } from "../../src/history-tree/client.ts";
import { SerializedForkHistoryPort } from "../../src/history-tree/serialized.ts";
import type { ForkHistoryPort } from "../../src/history-tree/ports.ts";
import { branchPage, changed, clone, committed, continuationPage, pathPage, removal, snapshot } from "./support.ts";

function port(): ForkHistoryPort { return { snapshot: async () => clone(snapshot), path: async () => clone(pathPage), branches: async () => clone(branchPage), continuations: async () => clone(continuationPage), deleteContinuation: async () => clone(removal), navigate: async () => clone(committed), listen: (listener) => { listener(clone(changed)); return () => {}; }, nextPlanId: () => "plan:test" }; }
describe("fork-history direct and serialized conformance", () => {
  for (const [name, value] of [["direct", port()], ["serialized", new SerializedForkHistoryPort(port())]] as const) it(name, async () => {
    const client = new ForkHistoryClient(value); expect(await client.snapshot()).toEqual(snapshot);
    expect(await client.path({ protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", expectedRevision: 4, target: { kind: "default" }, offset: 0, limit: 2 })).toEqual(pathPage);
    expect(await client.branches({ protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", expectedRevision: 4, offset: 0, limit: 2 })).toEqual(branchPage);
  });
  it("rejects future protocols and payload fields", async () => {
    const future = port(); future.snapshot = async () => ({ ...snapshot, protocolVersion: 2 }); await expect(new ForkHistoryClient(future).snapshot()).rejects.toThrow("exact protocol");
    const payload = port(); payload.snapshot = async () => ({ ...snapshot, payload: {} }); await expect(new ForkHistoryClient(payload).snapshot()).rejects.toThrow("payload");
  });
  it("rejects page requests above the generated hard ceiling", async () => {
    await expect(new ForkHistoryClient(port()).path({ protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", expectedRevision: 4, target: { kind: "default" }, offset: 0, limit: 257 })).rejects.toThrow("maximum is 256");
  });
});
