import { ForkHistoryController } from "@inflatable-cookie/longhorn-history-tree";
import { FORK_HISTORY_CHANGED_EVENT, createTauriForkHistoryPort } from "@inflatable-cookie/longhorn-history-tree/tauri";
import fixtureJson from "./fixture.json";

const fixture = structuredClone(fixtureJson) as any;
let listener: ((event: unknown) => void) | undefined;
let unlistenCount = 0;
const transport = {
  async invoke(command: string): Promise<unknown> {
    if (command === "longhorn_history_tree_snapshot") return structuredClone(fixture.rendererFixture.snapshot);
    if (command === "longhorn_history_tree_path") return structuredClone(fixture.rendererFixture.path);
    if (command === "longhorn_history_tree_branches") return structuredClone(fixture.rendererFixture.branches);
    if (command === "longhorn_history_tree_navigate") return structuredClone(fixture.rendererFixture.navigationResult);
    throw new Error(`unexpected command ${command}`);
  },
  async listen(event: string, next: (event: unknown) => void): Promise<() => void> {
    if (event !== FORK_HISTORY_CHANGED_EVENT) throw new Error(`unexpected event ${event}`);
    listener = next;
    return () => { listener = undefined; unlistenCount += 1; };
  },
};
const controller = new ForkHistoryController({ port: createTauriForkHistoryPort({ transport, nextPlanId: () => "plan:loophole-renderer" }), pathPageSize: 17, branchPageSize: 17 });
await controller.start();
await controller.loadBranches();
const navigation = fixture.rendererFixture.navigationResult;
const publicTrace = trace(controller.snapshot, controller.path, controller.branches, navigation);
if (!equalJson(publicTrace, fixture.publicTrace)) throw new Error("Loophole native and renderer traces diverged");
listener?.(structuredClone(fixture.rendererFixture.changedEvent));
await controller.stop();
if (unlistenCount !== 1 || listener !== undefined) throw new Error("Loophole controller leaked its listener");
console.log(JSON.stringify({ shape: "loophole", publicTrace, transport: "tauri", teardown: { unlistenCount }, cleanInstall: true }));

function trace(snapshot: any, path: any, branches: any, navigation: any) {
  const receipt = navigation.status === "committed" ? navigation.receipt : undefined;
  const moved = receipt?.movedEntryIds ?? [];
  return { historyId: snapshot.summary.historyId, revision: snapshot.summary.revision, currentBranchId: snapshot.summary.currentBranchId, currentEntryId: snapshot.summary.currentEntryId, undoDepth: snapshot.summary.undoDepth, redoDepth: snapshot.summary.redoDepth, retainedEntryCount: snapshot.summary.retainedEntryCount, branchCount: snapshot.summary.branchCount, alternatePathCount: snapshot.summary.alternatePathCount, pathEntryIds: path.entries.map((entry: any) => entry.entryId), branchIds: branches.branches.map((branch: any) => branch.branchId), movedEntryCount: moved.length, firstMovedEntryId: moved[0] ?? null, lastMovedEntryId: moved.at(-1) ?? null };
}

function equalJson(left: unknown, right: unknown): boolean { return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right)); }
function canonical(value: unknown): unknown { if (Array.isArray(value)) return value.map(canonical); if (value !== null && typeof value === "object") return Object.fromEntries(Object.entries(value).sort(([left], [right]) => left.localeCompare(right)).map(([key, entry]) => [key, canonical(entry)])); return value; }
