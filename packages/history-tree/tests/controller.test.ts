import { describe, expect, it } from "vitest";
import { ForkHistoryController } from "../src/controller.ts";
import type { ForkHistoryPort } from "../src/ports.ts";
import { branchPage, clone, committed, pathPage, snapshot } from "./support.ts";
function tracked() { const calls: string[] = []; const listeners = new Set<(value: unknown) => void>(); const port: ForkHistoryPort = { snapshot: async () => { calls.push("snapshot"); return clone(snapshot); }, path: async (command) => { calls.push(`path:${command.target.kind}`); return { ...clone(pathPage), branchId: command.target.kind === "branch" ? command.target.branchId : null }; }, branches: async () => { calls.push("branches"); return clone(branchPage); }, navigate: async () => { calls.push("navigate"); return clone(committed); }, listen: (listener) => { calls.push("listen"); listeners.add(listener); return () => { listeners.delete(listener); }; }, nextPlanId: () => "plan:test" }; return { port, calls, listeners }; }
describe("fork-history controller", () => {
  it("starts listener-first and leaves alternates unloaded", async () => { const state = tracked(); const controller = new ForkHistoryController({ port: state.port, pathPageSize: 2 }); await controller.start(); expect(state.calls).toEqual(["listen", "snapshot", "path:default"]); expect(controller.branches).toBeUndefined(); expect(controller.entries).toHaveLength(2); await controller.stop(); expect(state.listeners.size).toBe(0); });
  it("loads branch metadata and alternate paths only after explicit requests", async () => { const state = tracked(); const controller = new ForkHistoryController({ port: state.port }); await controller.start(); await controller.loadBranches(); await controller.selectBranchPath("branch:alternate"); expect(state.calls).toContain("branches"); expect(state.calls.at(-2)).toBe("snapshot"); expect(state.calls.at(-1)).toBe("path:branch"); await controller.stop(); });
  it("disposes a listener whose async registration resolves after stop", async () => {
    let resolve!: (dispose: () => void) => void; let disposed = 0;
    const state = tracked(); state.port.listen = () => new Promise((accept) => { resolve = accept; });
    const controller = new ForkHistoryController({ port: state.port }); const starting = controller.start(); await Promise.resolve(); const stopping = controller.stop(); resolve(() => { disposed += 1; }); await Promise.all([starting, stopping]); expect(disposed).toBe(1); expect(controller.status.kind).toBe("idle");
  });
});
