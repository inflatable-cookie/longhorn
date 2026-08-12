import { describe, expect, it } from "bun:test";
import { ForkHistoryController } from "../../src/history-tree/controller.ts";
import type { ForkHistoryPort } from "../../src/history-tree/ports.ts";
import { branchPage, changed, clone, committed, continuationPage, pathPage, snapshot } from "./support.ts";
function tracked() {
  const calls: string[] = []; const branchOffsets: number[] = []; const listeners = new Set<(value: unknown) => void>();
  let revision = 4;
  const at = <T extends { revision: number }>(value: T): T => ({ ...value, revision });
  const port: ForkHistoryPort = {
    snapshot: async () => { calls.push("snapshot"); const value = clone(snapshot); return { ...value, summary: { ...value.summary, revision } }; },
    path: async (command) => { calls.push(`path:${command.target.kind}`); return at({ ...clone(pathPage), branchId: command.target.kind === "branch" ? command.target.branchId : null }); },
    branches: async (command) => { calls.push("branches"); branchOffsets.push(command.offset); return at({ ...clone(branchPage), offset: command.offset }); },
    continuations: async (command) => { calls.push("continuations"); return at({ ...clone(continuationPage), anchorEntryId: command.anchorEntryId, offset: command.offset }); },
    navigate: async () => { calls.push("navigate"); return clone(committed); },
    listen: (listener) => { calls.push("listen"); listeners.add(listener); return () => { listeners.delete(listener); }; },
    nextPlanId: () => "plan:test",
  };
  return { port, calls, listeners, branchOffsets, advance: (next: number) => { revision = next; } };
}

async function waitFor(condition: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 50; attempt += 1) { if (condition()) return; await new Promise((resolve) => setTimeout(resolve, 1)); }
  throw new Error("condition never held");
}
describe("fork-history controller", () => {
  it("rejects controller page sizes above the generated hard ceiling", () => { expect(() => new ForkHistoryController({ port: tracked().port, pathPageSize: 257 })).toThrow("1..256"); });
  it("starts listener-first and leaves alternates unloaded", async () => { const state = tracked(); const controller = new ForkHistoryController({ port: state.port, pathPageSize: 2 }); await controller.start(); expect(state.calls).toEqual(["listen", "snapshot", "path:default"]); expect(controller.branches).toBeUndefined(); expect(controller.entries).toHaveLength(2); await controller.stop(); expect(state.listeners.size).toBe(0); });
  it("loads branch metadata and alternate paths only after explicit requests", async () => { const state = tracked(); const controller = new ForkHistoryController({ port: state.port }); await controller.start(); expect(state.calls).not.toContain("branches"); await controller.loadBranches(); await controller.selectBranchPath("branch:alternate"); expect(state.calls.slice(-3)).toEqual(["snapshot", "path:branch", "branches"]); await controller.stop(); });

  // A consumer that loads branches once and keeps editing used to lose the page
  // on every mutation: `#install` discarded it as stale, and `loadBranches`
  // threw a projection gap on the first mismatch. Loophole worked around it
  // with an event-driven reload and a retry through the gap.
  it("carries the branches page through a change rather than dropping it", async () => {
    const state = tracked();
    const controller = new ForkHistoryController({ port: state.port });
    await controller.start();
    await controller.loadBranches();
    expect(controller.branches?.revision).toBe(4);

    // The authority moves, and says so.
    state.advance(5);
    for (const listener of state.listeners) listener(clone({ ...changed, committedRevision: 5 }));
    await waitFor(() => controller.branches?.revision === 5);

    expect(controller.branches?.revision).toBe(5);
    expect(controller.snapshot?.summary.revision).toBe(5);
    await controller.stop();
  });

  it("returns the branch page the consumer was reading, not the first one", async () => {
    const state = tracked();
    const controller = new ForkHistoryController({ port: state.port, branchPageSize: 1 });
    await controller.start();
    await controller.loadBranches(1);
    expect(state.branchOffsets.at(-1)).toBe(1);

    state.advance(5);
    for (const listener of state.listeners) listener(clone({ ...changed, committedRevision: 5 }));
    await waitFor(() => controller.branches?.revision === 5);

    // Two fetches, and the second asked for the page being read rather than
    // starting over at the first one.
    expect(state.branchOffsets).toEqual([1, 1]);
    expect(controller.branches?.offset).toBe(1);
    await controller.stop();
  });
  it("disposes a listener whose async registration resolves after stop", async () => {
    let resolve!: (dispose: () => void) => void; let disposed = 0;
    const state = tracked(); state.port.listen = () => new Promise((accept) => { resolve = accept; });
    const controller = new ForkHistoryController({ port: state.port }); const starting = controller.start(); await Promise.resolve(); const stopping = controller.stop(); resolve(() => { disposed += 1; }); await Promise.all([starting, stopping]); expect(disposed).toBe(1); expect(controller.status.kind).toBe("idle");
  });
});
