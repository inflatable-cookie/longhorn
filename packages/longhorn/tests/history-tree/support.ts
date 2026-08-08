import type { ForkBranchPageSnapshot, ForkChangedEvent, ForkNavigationResult, ForkPathPageSnapshot, ForkSnapshot } from "../../src/history-tree/generated/protocol.ts";
export const snapshot: ForkSnapshot = { protocolVersion: 1, authorityEpoch: 7, summary: { historyId: "history:tree", revision: 4, currentBranchId: "branch:main", currentEntryId: "entry:b", undoDepth: 2, redoDepth: 1, nextUndoLabel: "Move panel", nextRedoLabel: "Resize region", retainedEntryCount: 4, retainedEncodedWeight: 64, branchCount: 2, alternatePathCount: 2 } };
export const pathPage: ForkPathPageSnapshot = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", revision: 4, branchId: null, headEntryId: "entry:c", offset: 0, totalEntries: 3, entries: [
  { entryId: "entry:c", label: "Resize region", kindId: "fixture:layout", groupId: null, sequence: 3, committedRevision: 3, encodedWeight: 16, position: "future" },
  { entryId: "entry:b", label: "Move panel", kindId: "fixture:layout", groupId: null, sequence: 2, committedRevision: 2, encodedWeight: 16, position: "current" },
], truncatedBefore: false, truncatedAfter: true };
export const branchPage: ForkBranchPageSnapshot = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", revision: 4, offset: 0, totalBranches: 2, branches: [
  { branchId: "branch:alternate", headEntryId: "entry:d", divergenceEntryId: "entry:b", name: "Alternate", annotation: null, pinned: false, current: false },
  { branchId: "branch:main", headEntryId: "entry:c", divergenceEntryId: "entry:c", name: "Main", annotation: null, pinned: true, current: true },
], truncatedBefore: false, truncatedAfter: false };
export const committedSnapshot: ForkSnapshot = { ...snapshot, summary: { ...snapshot.summary, revision: 5, currentBranchId: "branch:alternate", currentEntryId: "entry:d", undoDepth: 3, redoDepth: 0, nextUndoLabel: "Alternate edit", nextRedoLabel: null } };
export const committed: ForkNavigationResult = { status: "committed", snapshot: committedSnapshot, receipt: { historyId: "history:tree", planId: "plan:fixture", previousRevision: 4, committedRevision: 5, sourceEntryId: "entry:b", targetEntryId: "entry:d", targetBranchId: "branch:alternate", movedEntryIds: ["entry:d"] } };
export const changed: ForkChangedEvent = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", previousRevision: 4, committedRevision: 5, kind: "navigation" };
export function clone<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
