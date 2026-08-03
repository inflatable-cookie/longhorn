import {
  FORK_HISTORY_CHANGED_KINDS,
  FORK_HISTORY_ENTRY_POSITIONS,
  FORK_HISTORY_NAVIGATION_STATUSES,
  FORK_HISTORY_NAVIGATION_TARGETS,
  FORK_HISTORY_NAVIGATION_REJECTION_CODES,
  FORK_HISTORY_PATH_TARGETS,
  FORK_HISTORY_PROTOCOL_VERSION,
  type ForkBranchPageCommand,
  type ForkBranchPageSnapshot,
  type ForkChangedEvent,
  type ForkNavigationCommand,
  type ForkNavigationResult,
  type ForkPathPageCommand,
  type ForkPathPageSnapshot,
  type ForkSnapshot,
} from "./generated/protocol.ts";

export class ForkHistoryCompatibilityError extends Error {
  constructor(readonly path: string, message: string) {
    super(`${path}: ${message}`);
    this.name = "ForkHistoryCompatibilityError";
  }
}

export function assertForkSnapshot(value: unknown): asserts value is ForkSnapshot {
  noPayload(value);
  const root = object(value, "$");
  exact(root, "$", ["protocolVersion", "authorityEpoch", "summary"]);
  protocol(root.protocolVersion, "$.protocolVersion");
  positive(root.authorityEpoch, "$.authorityEpoch");
  const summary = object(root.summary, "$.summary");
  exact(summary, "$.summary", ["historyId", "revision", "currentBranchId", "currentEntryId", "undoDepth", "redoDepth", "nextUndoLabel", "nextRedoLabel", "retainedEntryCount", "retainedEncodedWeight", "branchCount", "alternatePathCount"]);
  id(summary.historyId, "$.summary.historyId");
  integer(summary.revision, "$.summary.revision");
  id(summary.currentBranchId, "$.summary.currentBranchId");
  optionalId(summary.currentEntryId, "$.summary.currentEntryId");
  for (const key of ["undoDepth", "redoDepth", "retainedEntryCount", "retainedEncodedWeight", "branchCount", "alternatePathCount"] as const) integer(summary[key], `$.summary.${key}`);
  optionalString(summary.nextUndoLabel, "$.summary.nextUndoLabel");
  optionalString(summary.nextRedoLabel, "$.summary.nextRedoLabel");
}

export function assertForkPathCommand(value: unknown): asserts value is ForkPathPageCommand {
  commandBase(value, ["target"]);
  const target = object(object(value, "$").target, "$.target");
  oneOf(target.kind, "$.target.kind", FORK_HISTORY_PATH_TARGETS);
  exact(target, "$.target", target.kind === "branch" ? ["kind", "branchId"] : ["kind"]);
  if (target.kind === "branch") id(target.branchId, "$.target.branchId");
}

export function assertForkBranchCommand(value: unknown): asserts value is ForkBranchPageCommand {
  commandBase(value, []);
}

export function assertForkPathPage(value: unknown): asserts value is ForkPathPageSnapshot {
  noPayload(value);
  const root = object(value, "$");
  exact(root, "$", ["protocolVersion", "authorityEpoch", "historyId", "revision", "branchId", "headEntryId", "offset", "totalEntries", "entries", "truncatedBefore", "truncatedAfter"]);
  snapshotBase(root);
  optionalId(root.branchId, "$.branchId");
  optionalId(root.headEntryId, "$.headEntryId");
  integer(root.totalEntries, "$.totalEntries");
  array(root.entries, "$.entries").forEach((entry, index) => entryRecord(entry, `$.entries[${index}]`));
  boolean(root.truncatedBefore, "$.truncatedBefore");
  boolean(root.truncatedAfter, "$.truncatedAfter");
}

export function assertForkBranchPage(value: unknown): asserts value is ForkBranchPageSnapshot {
  noPayload(value);
  const root = object(value, "$");
  exact(root, "$", ["protocolVersion", "authorityEpoch", "historyId", "revision", "offset", "totalBranches", "branches", "truncatedBefore", "truncatedAfter"]);
  snapshotBase(root);
  integer(root.totalBranches, "$.totalBranches");
  array(root.branches, "$.branches").forEach((value, index) => {
    const branch = object(value, `$.branches[${index}]`);
    exact(branch, `$.branches[${index}]`, ["branchId", "headEntryId", "divergenceEntryId", "name", "annotation", "pinned", "current"]);
    id(branch.branchId, `$.branches[${index}].branchId`);
    optionalId(branch.headEntryId, `$.branches[${index}].headEntryId`);
    optionalId(branch.divergenceEntryId, `$.branches[${index}].divergenceEntryId`);
    optionalString(branch.name, `$.branches[${index}].name`);
    optionalString(branch.annotation, `$.branches[${index}].annotation`);
    boolean(branch.pinned, `$.branches[${index}].pinned`);
    boolean(branch.current, `$.branches[${index}].current`);
  });
  boolean(root.truncatedBefore, "$.truncatedBefore");
  boolean(root.truncatedAfter, "$.truncatedAfter");
}

export function assertForkNavigationCommand(value: unknown): asserts value is ForkNavigationCommand {
  noPayload(value);
  const root = object(value, "$");
  exact(root, "$", ["protocolVersion", "authorityEpoch", "historyId", "planId", "expectedRevision", "target"]);
  protocol(root.protocolVersion, "$.protocolVersion"); positive(root.authorityEpoch, "$.authorityEpoch"); id(root.historyId, "$.historyId"); id(root.planId, "$.planId"); integer(root.expectedRevision, "$.expectedRevision");
  const target = object(root.target, "$.target"); oneOf(target.kind, "$.target.kind", FORK_HISTORY_NAVIGATION_TARGETS);
  exact(target, "$.target", target.kind === "checkout" ? ["kind", "branchId", "entryId"] : ["kind"]);
  if (target.kind === "checkout") { id(target.branchId, "$.target.branchId"); id(target.entryId, "$.target.entryId"); }
}

export function assertForkNavigationResult(value: unknown): asserts value is ForkNavigationResult {
  noPayload(value);
  const root = object(value, "$"); oneOf(root.status, "$.status", FORK_HISTORY_NAVIGATION_STATUSES);
  exact(root, "$", root.status === "committed" ? ["status", "snapshot", "receipt"] : ["status", "snapshot", "rejection"]);
  assertForkSnapshot(root.snapshot);
  if (root.status === "committed") {
    const receipt = object(root.receipt, "$.receipt");
    exact(receipt, "$.receipt", ["historyId", "planId", "previousRevision", "committedRevision", "sourceEntryId", "targetEntryId", "targetBranchId", "movedEntryIds"]);
    id(receipt.historyId, "$.receipt.historyId"); id(receipt.planId, "$.receipt.planId"); integer(receipt.previousRevision, "$.receipt.previousRevision"); integer(receipt.committedRevision, "$.receipt.committedRevision"); optionalId(receipt.sourceEntryId, "$.receipt.sourceEntryId"); optionalId(receipt.targetEntryId, "$.receipt.targetEntryId"); id(receipt.targetBranchId, "$.receipt.targetBranchId"); array(receipt.movedEntryIds, "$.receipt.movedEntryIds").forEach((entry, index) => id(entry, `$.receipt.movedEntryIds[${index}]`));
  } else {
    const rejection = object(root.rejection, "$.rejection");
    exact(rejection, "$.rejection", ["code", "detail", "refreshRequired"]); oneOf(rejection.code, "$.rejection.code", FORK_HISTORY_NAVIGATION_REJECTION_CODES); string(rejection.detail, "$.rejection.detail"); boolean(rejection.refreshRequired, "$.rejection.refreshRequired");
  }
}

export function assertForkChangedEvent(value: unknown): asserts value is ForkChangedEvent {
  noPayload(value); const root = object(value, "$");
  exact(root, "$", ["protocolVersion", "authorityEpoch", "historyId", "previousRevision", "committedRevision", "kind"]);
  protocol(root.protocolVersion, "$.protocolVersion"); positive(root.authorityEpoch, "$.authorityEpoch"); id(root.historyId, "$.historyId");
  if (root.previousRevision !== null) integer(root.previousRevision, "$.previousRevision");
  integer(root.committedRevision, "$.committedRevision"); oneOf(root.kind, "$.kind", FORK_HISTORY_CHANGED_KINDS);
}

function commandBase(value: unknown, extra: string[]): void { noPayload(value); const root = object(value, "$"); exact(root, "$", ["protocolVersion", "authorityEpoch", "historyId", "expectedRevision", ...extra, "offset", "limit"]); protocol(root.protocolVersion, "$.protocolVersion"); positive(root.authorityEpoch, "$.authorityEpoch"); id(root.historyId, "$.historyId"); integer(root.expectedRevision, "$.expectedRevision"); integer(root.offset, "$.offset"); positive(root.limit, "$.limit"); }
function snapshotBase(root: Record<string, unknown>): void { protocol(root.protocolVersion, "$.protocolVersion"); positive(root.authorityEpoch, "$.authorityEpoch"); id(root.historyId, "$.historyId"); integer(root.revision, "$.revision"); integer(root.offset, "$.offset"); }
function entryRecord(value: unknown, path: string): void { const root = object(value, path); exact(root, path, ["entryId", "label", "kindId", "groupId", "sequence", "committedRevision", "encodedWeight", "position"]); id(root.entryId, `${path}.entryId`); string(root.label, `${path}.label`); optionalId(root.kindId, `${path}.kindId`); optionalId(root.groupId, `${path}.groupId`); positive(root.sequence, `${path}.sequence`); integer(root.committedRevision, `${path}.committedRevision`); integer(root.encodedWeight, `${path}.encodedWeight`); oneOf(root.position, `${path}.position`, FORK_HISTORY_ENTRY_POSITIONS); }
function noPayload(value: unknown): void { const visit = (candidate: unknown, path: string): void => { if (Array.isArray(candidate)) return candidate.forEach((item, index) => visit(item, `${path}[${index}]`)); if (candidate !== null && typeof candidate === "object") for (const [key, child] of Object.entries(candidate)) { if (key.toLocaleLowerCase().includes("payload")) fail(`${path}.${key}`, "product payload field is forbidden"); visit(child, `${path}.${key}`); } }; visit(value, "$"); }
function object(value: unknown, path: string): Record<string, unknown> { if (value === null || typeof value !== "object" || Array.isArray(value)) fail(path, "expected object"); return value as Record<string, unknown>; }
function array(value: unknown, path: string): unknown[] { if (!Array.isArray(value)) fail(path, "expected array"); return value; }
function exact(value: Record<string, unknown>, path: string, expected: string[]): void { const actual = Object.keys(value).sort(); const wanted = [...expected].sort(); if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) fail(path, `unexpected keys: ${actual.join(",")}`); }
function protocol(value: unknown, path: string): void { if (value !== FORK_HISTORY_PROTOCOL_VERSION) fail(path, `expected exact protocol ${FORK_HISTORY_PROTOCOL_VERSION}`); }
function integer(value: unknown, path: string): void { if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) fail(path, "expected non-negative safe integer"); }
function positive(value: unknown, path: string): void { integer(value, path); if (value === 0) fail(path, "expected positive integer"); }
function string(value: unknown, path: string): void { if (typeof value !== "string") fail(path, "expected string"); }
function id(value: unknown, path: string): void { string(value, path); if ((value as string).length === 0) fail(path, "expected non-empty id"); }
function optionalId(value: unknown, path: string): void { if (value !== null) id(value, path); }
function optionalString(value: unknown, path: string): void { if (value !== null) string(value, path); }
function boolean(value: unknown, path: string): void { if (typeof value !== "boolean") fail(path, "expected boolean"); }
function oneOf(value: unknown, path: string, values: readonly string[]): void { if (typeof value !== "string" || !values.includes(value)) fail(path, "unsupported value"); }
function fail(path: string, message: string): never { throw new ForkHistoryCompatibilityError(path, message); }
