import type { ForkBranchPageSnapshot, ForkChangedEvent, ForkContinuationPageSnapshot, ForkRemovalReceiptProjection, ForkNavigationResult, ForkPathPageSnapshot, ForkSnapshot } from "../../src/history-tree/generated/protocol.ts";
export const snapshot: ForkSnapshot = { protocolVersion: 1, authorityEpoch: 7, summary: { historyId: "history:tree", revision: 4, currentBranchId: "branch:main", currentEntryId: "entry:b", undoDepth: 2, redoDepth: 1, nextUndoLabel: "Move panel", nextRedoLabel: "Resize region", retainedEntryCount: 4, retainedEncodedWeight: 64, branchCount: 2, alternatePathCount: 2 } };
export const pathPage: ForkPathPageSnapshot = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", revision: 4, branchId: null, headEntryId: "entry:c", precedingContinuationCount: 1, offset: 0, totalEntries: 3, entries: [
  { entryId: "entry:c", label: "Resize region", kindId: "fixture:layout", groupId: null, recordedAt: null, continuationCount: 1, sequence: 3, committedRevision: 3, encodedWeight: 16, position: "future" },
  { entryId: "entry:b", label: "Move panel", kindId: "fixture:layout", groupId: null, recordedAt: 1765432100000, continuationCount: 2, sequence: 2, committedRevision: 2, encodedWeight: 16, position: "current" },
], truncatedBefore: false, truncatedAfter: true };
export const branchPage: ForkBranchPageSnapshot = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", revision: 4, offset: 0, totalBranches: 2, branches: [
  { branchId: "branch:alternate", headEntryId: "entry:d", divergenceEntryId: "entry:b", divergenceBranchId: "branch:main", name: "Alternate", annotation: null, pinned: false, current: false },
  { branchId: "branch:main", headEntryId: "entry:c", divergenceEntryId: "entry:c", divergenceBranchId: null, name: "Main", annotation: null, pinned: true, current: true },
], truncatedBefore: false, truncatedAfter: false };
export const committedSnapshot: ForkSnapshot = { ...snapshot, summary: { ...snapshot.summary, revision: 5, currentBranchId: "branch:alternate", currentEntryId: "entry:d", undoDepth: 3, redoDepth: 0, nextUndoLabel: "Alternate edit", nextRedoLabel: null } };
export const committed: ForkNavigationResult = { status: "committed", snapshot: committedSnapshot, receipt: { historyId: "history:tree", planId: "plan:fixture", previousRevision: 4, committedRevision: 5, sourceEntryId: "entry:b", targetEntryId: "entry:d", targetBranchId: "branch:alternate", movedEntryIds: ["entry:d"] } };
export const changed: ForkChangedEvent = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", previousRevision: 4, committedRevision: 5, kind: "navigation" };
export function clone<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
export const continuationPage: ForkContinuationPageSnapshot = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", revision: 4, anchorEntryId: "entry:b", offset: 0, totalContinuations: 2, continuations: [
  { entryId: "entry:c", label: "Resize region", recordedAt: null, preferred: true, entryCount: 1, branchId: "branch:main", branchName: "Main" },
  { entryId: "entry:d", label: "Alternate edit", recordedAt: 1765432100000, preferred: false, entryCount: 1, branchId: "branch:alternate", branchName: "Alternate" },
], truncatedBefore: false, truncatedAfter: false };
export const removal: ForkRemovalReceiptProjection = { protocolVersion: 1, authorityEpoch: 7, historyId: "history:tree", previousRevision: 4, committedRevision: 5, removedEntries: [
  { entryId: "entry:d", sequence: 4, encodedWeight: 16 },
], removedBranches: ["branch:alternate"], removedCheckpoints: [], retainedEntryCount: 3, retainedEncodedWeight: 48 };
