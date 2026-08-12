import { ForkHistoryClient } from "@inflatable-cookie/longhorn/history-tree";
import fixtureJson from "./fixture.json";

const fixture = structuredClone(fixtureJson) as any;
const client = new ForkHistoryClient({
  async snapshot() { return structuredClone(fixture.rendererFixture.snapshot); },
  async path() { return structuredClone(fixture.rendererFixture.path); },
  async branches() { return structuredClone(fixture.rendererFixture.branches); },
  async continuations() { return structuredClone(fixture.rendererFixture.continuations); },
  async deleteContinuation(): Promise<never> { throw new Error("the document proof does not exercise deletion"); },
  async prune(): Promise<never> { throw new Error("the document proof does not exercise pruning"); },
  async navigate() { return structuredClone(fixture.rendererFixture.navigationResult); },
  nextPlanId() { return "plan:document-renderer"; },
});

const snapshot = await client.snapshot();
const path = await client.path({ protocolVersion: 1, authorityEpoch: snapshot.authorityEpoch, historyId: snapshot.summary.historyId, expectedRevision: snapshot.summary.revision, target: { kind: "default" }, offset: 0, limit: 17 });
const branches = await client.branches({ protocolVersion: 1, authorityEpoch: snapshot.authorityEpoch, historyId: snapshot.summary.historyId, expectedRevision: snapshot.summary.revision, offset: 0, limit: 17 });
const continuations = await client.continuations({ protocolVersion: 1, authorityEpoch: snapshot.authorityEpoch, historyId: snapshot.summary.historyId, expectedRevision: snapshot.summary.revision, anchorEntryId: fixture.rendererFixture.continuations.anchorEntryId, offset: 0, limit: 17 });
const navigation = await client.navigate({ protocolVersion: 1, authorityEpoch: snapshot.authorityEpoch, historyId: snapshot.summary.historyId, expectedRevision: snapshot.summary.revision, planId: client.nextPlanId(), target: { kind: "redo" } });
const publicTrace = trace(snapshot, path, branches, continuations, navigation);
if (!equalJson(publicTrace, fixture.publicTrace)) throw new Error("document native and renderer traces diverged");
console.log(JSON.stringify({ shape: "document", publicTrace, transport: "direct", cleanInstall: true }));

function trace(snapshot: any, path: any, branches: any, continuations: any, navigation: any) {
  const receipt = navigation.status === "committed" ? navigation.receipt : undefined;
  const moved = receipt?.movedEntryIds ?? [];
  return { historyId: snapshot.summary.historyId, revision: snapshot.summary.revision, currentBranchId: snapshot.summary.currentBranchId, currentEntryId: snapshot.summary.currentEntryId, undoDepth: snapshot.summary.undoDepth, redoDepth: snapshot.summary.redoDepth, retainedEntryCount: snapshot.summary.retainedEntryCount, branchCount: snapshot.summary.branchCount, alternatePathCount: snapshot.summary.alternatePathCount, pathEntryIds: path.entries.map((entry: any) => entry.entryId), branchIds: branches.branches.map((branch: any) => branch.branchId), continuationAnchorId: continuations.anchorEntryId, continuationEntryIds: continuations.continuations.map((continuation: any) => continuation.entryId), movedEntryCount: moved.length, firstMovedEntryId: moved[0] ?? null, lastMovedEntryId: moved.at(-1) ?? null };
}

function equalJson(left: unknown, right: unknown): boolean { return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right)); }
function canonical(value: unknown): unknown { if (Array.isArray(value)) return value.map(canonical); if (value !== null && typeof value === "object") return Object.fromEntries(Object.entries(value).sort(([left], [right]) => left.localeCompare(right)).map(([key, entry]) => [key, canonical(entry)])); return value; }
