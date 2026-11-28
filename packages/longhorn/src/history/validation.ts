import { isOpaqueId } from "../core/identifiers.ts";
import { checkWireFields, readWireRecord, wireVariantFields } from "../core/record.ts";
import {
  HISTORY_CHANGED_KINDS,
  HISTORY_ENTRY_POSITIONS,
  HISTORY_PAGE_FLOORS,
  HISTORY_MODES,
  HISTORY_NAVIGATION_DIRECTIONS,
  HISTORY_NAVIGATION_REJECTION_CODES,
  HISTORY_NAVIGATION_STATUSES,
  HISTORY_NAVIGATION_TARGETS,
  HISTORY_PROTOCOL_VERSION,
} from "./generated/protocol.ts";
import type {
  HistoryChangedEvent,
  HistoryNavigationCommand,
  HistoryNavigationResult,
  HistoryPageCommand,
  HistoryPageSnapshot,
  HistorySnapshot,
} from "./generated/protocol.ts";
import { HISTORY_FIELDS } from "./generated/fields.ts";
import {
  HISTORY_VARIANT_FIELDS,
  HISTORY_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";

export class HistoryProtocolValidationError extends Error {
  constructor(
    readonly path: string,
    message: string,
  ) {
    super(`${path}: ${message}`);
    this.name = "HistoryProtocolValidationError";
  }
}

export function assertValidHistorySnapshot(value: unknown): asserts value is HistorySnapshot {
  assertNoPayload(value, "$");
  const record = object(value, "$");
  keys(record, "$", HISTORY_FIELDS.HistorySnapshot);
  protocol(record.protocolVersion, "$.protocolVersion");
  positiveInteger(record.authorityEpoch, "$.authorityEpoch");
  summary(record.summary, "$.summary");
}

export function assertValidHistoryPageCommand(value: unknown): asserts value is HistoryPageCommand {
  assertNoPayload(value, "$");
  const record = object(value, "$");
  keys(record, "$", HISTORY_FIELDS.HistoryPageCommand);
  protocol(record.protocolVersion, "$.protocolVersion");
  positiveInteger(record.authorityEpoch, "$.authorityEpoch");
  id(record.historyId, "$.historyId");
  integer(record.expectedRevision, "$.expectedRevision");
  integer(record.offset, "$.offset");
  positiveInteger(record.limit, "$.limit");
}

export function assertValidHistoryPageSnapshot(
  value: unknown,
): asserts value is HistoryPageSnapshot {
  assertNoPayload(value, "$");
  const record = object(value, "$");
  keys(record, "$", HISTORY_FIELDS.HistoryPageSnapshot);
  protocol(record.protocolVersion, "$.protocolVersion");
  positiveInteger(record.authorityEpoch, "$.authorityEpoch");
  id(record.historyId, "$.historyId");
  integer(record.revision, "$.revision");
  integer(record.offset, "$.offset");
  integer(record.totalEntries, "$.totalEntries");
  boolean(record.truncatedBefore, "$.truncatedBefore");
  boolean(record.truncatedAfter, "$.truncatedAfter");
  baseline(record.retainedBaseline, "$.retainedBaseline");
  floor(record.floor, "$.floor");
  array(record.entries, "$.entries").forEach((entry, index) =>
    entryRecord(entry, `$.entries[${index}]`),
  );
}

export function assertValidHistoryNavigationCommand(
  value: unknown,
): asserts value is HistoryNavigationCommand {
  assertNoPayload(value, "$");
  const record = object(value, "$");
  keys(record, "$", HISTORY_FIELDS.HistoryNavigationCommand);
  protocol(record.protocolVersion, "$.protocolVersion");
  positiveInteger(record.authorityEpoch, "$.authorityEpoch");
  id(record.historyId, "$.historyId");
  id(record.planId, "$.planId");
  integer(record.expectedRevision, "$.expectedRevision");
  const target = object(record.target, "$.target");
  oneOf(target.kind, "$.target.kind", HISTORY_NAVIGATION_TARGETS);
  keys(target, "$.target", variantKeys("HistoryNavigationTargetProjection", target, "$.target"));
  if (target.kind === "checkout") {
    id(target.entryId, "$.target.entryId");
  }
}

export function assertValidHistoryNavigationResult(
  value: unknown,
): asserts value is HistoryNavigationResult {
  assertNoPayload(value, "$");
  const record = object(value, "$");
  oneOf(record.status, "$.status", HISTORY_NAVIGATION_STATUSES);
  keys(record, "$", variantKeys("HistoryNavigationResult", record, "$"));
  assertValidHistorySnapshot(record.snapshot);
  if (record.status === "committed") {
    receipt(record.receipt, "$.receipt");
  } else {
    rejection(record.rejection, "$.rejection");
  }
}

export function assertValidHistoryChangedEvent(
  value: unknown,
): asserts value is HistoryChangedEvent {
  assertNoPayload(value, "$");
  const record = object(value, "$");
  keys(record, "$", HISTORY_FIELDS.HistoryChangedEvent);
  protocol(record.protocolVersion, "$.protocolVersion");
  positiveInteger(record.authorityEpoch, "$.authorityEpoch");
  id(record.historyId, "$.historyId");
  optionalInteger(record.previousRevision, "$.previousRevision");
  integer(record.committedRevision, "$.committedRevision");
  oneOf(record.kind, "$.kind", HISTORY_CHANGED_KINDS);
}

function summary(value: unknown, path: string): void {
  const record = object(value, path);
  keys(record, path, HISTORY_FIELDS.HistorySummaryProjection);
  id(record.historyId, `${path}.historyId`);
  integer(record.revision, `${path}.revision`);
  oneOf(record.mode, `${path}.mode`, HISTORY_MODES);
  integer(record.undoDepth, `${path}.undoDepth`);
  integer(record.redoDepth, `${path}.redoDepth`);
  optionalId(record.currentEntryId, `${path}.currentEntryId`);
  optionalString(record.nextUndoLabel, `${path}.nextUndoLabel`);
  optionalString(record.nextRedoLabel, `${path}.nextRedoLabel`);
  integer(record.retainedEntryCount, `${path}.retainedEntryCount`);
  integer(record.retainedEncodedWeight, `${path}.retainedEncodedWeight`);
  baseline(record.retainedBaseline, `${path}.retainedBaseline`);
}

function baseline(value: unknown, path: string): void {
  const record = object(value, path);
  keys(record, path, HISTORY_FIELDS.HistoryBaselineProjection);
  integer(record.prunedEntryCount, `${path}.prunedEntryCount`);
  integer(record.prunedEncodedWeight, `${path}.prunedEncodedWeight`);
  optionalId(record.lastPrunedEntryId, `${path}.lastPrunedEntryId`);
  optionalInteger(record.lastPrunedSequence, `${path}.lastPrunedSequence`);
}

function entryRecord(value: unknown, path: string): void {
  const record = object(value, path);
  keys(record, path, HISTORY_FIELDS.HistoryEntryRecord);
  id(record.entryId, `${path}.entryId`);
  string(record.label, `${path}.label`);
  optionalId(record.kindId, `${path}.kindId`);
  optionalId(record.groupId, `${path}.groupId`);
  positiveInteger(record.sequence, `${path}.sequence`);
  integer(record.committedRevision, `${path}.committedRevision`);
  integer(record.encodedWeight, `${path}.encodedWeight`);
  oneOf(record.position, `${path}.position`, HISTORY_ENTRY_POSITIONS);
}

function receipt(value: unknown, path: string): void {
  const record = object(value, path);
  keys(record, path, HISTORY_FIELDS.HistoryNavigationReceiptProjection);
  id(record.historyId, `${path}.historyId`);
  id(record.planId, `${path}.planId`);
  integer(record.previousRevision, `${path}.previousRevision`);
  integer(record.committedRevision, `${path}.committedRevision`);
  oneOf(record.direction, `${path}.direction`, HISTORY_NAVIGATION_DIRECTIONS);
  array(record.movedEntryIds, `${path}.movedEntryIds`).forEach((entry, index) =>
    id(entry, `${path}.movedEntryIds[${index}]`),
  );
  navigationPosition(record.sourcePosition, `${path}.sourcePosition`);
  navigationPosition(record.authoritativePosition, `${path}.authoritativePosition`);
}

function navigationPosition(value: unknown, path: string): void {
  const record = object(value, path);
  keys(record, path, HISTORY_FIELDS.HistoryNavigationPositionProjection);
  integer(record.appliedDepth, `${path}.appliedDepth`);
  integer(record.futureDepth, `${path}.futureDepth`);
  optionalId(record.currentEntryId, `${path}.currentEntryId`);
  optionalString(record.nextUndoLabel, `${path}.nextUndoLabel`);
  optionalId(record.nextRedoEntryId, `${path}.nextRedoEntryId`);
  optionalString(record.nextRedoLabel, `${path}.nextRedoLabel`);
}

function rejection(value: unknown, path: string): void {
  const record = object(value, path);
  keys(record, path, HISTORY_FIELDS.HistoryNavigationRejectionProjection);
  oneOf(record.code, `${path}.code`, HISTORY_NAVIGATION_REJECTION_CODES);
  string(record.detail, `${path}.detail`);
  boolean(record.refreshRequired, `${path}.refreshRequired`);
}

function protocol(value: unknown, path: string): void {
  if (value !== HISTORY_PROTOCOL_VERSION) {
    fail(path, `expected exact protocol ${HISTORY_PROTOCOL_VERSION}`);
  }
}

function object(value: unknown, path: string): Record<string, unknown> {
  const result = readWireRecord(value, () => fail(path, "expected object"));
  return result;
}

function array(value: unknown, path: string): ReadonlyArray<unknown> {
  if (!Array.isArray(value)) {
    fail(path, "expected array");
  }
  return value;
}

function keys(value: Record<string, unknown>, path: string, allowed: ReadonlyArray<string>): void {
  checkWireFields(value, allowed, (code, key) =>
    fail(`${path}.${key}`, code === "unknown_field" ? "unknown field" : "missing field"),
  );
}

function oneOf(value: unknown, path: string, values: ReadonlyArray<string>): void {
  if (typeof value !== "string" || !values.includes(value)) {
    fail(path, `expected one of ${values.join(", ")}`);
  }
}

function id(value: unknown, path: string): void {
  if (!isOpaqueId(value)) {
    fail(path, "expected bounded lowercase opaque id");
  }
}

function optionalId(value: unknown, path: string): void {
  if (value !== null) {
    id(value, path);
  }
}

function string(value: unknown, path: string): asserts value is string {
  if (typeof value !== "string" || value.length === 0) {
    fail(path, "expected nonempty string");
  }
}

function optionalString(value: unknown, path: string): void {
  if (value !== null) {
    string(value, path);
  }
}

function integer(value: unknown, path: string): asserts value is number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    fail(path, "expected nonnegative safe integer");
  }
}

function positiveInteger(value: unknown, path: string): void {
  integer(value, path);
  if (value === 0) {
    fail(path, "expected nonzero integer");
  }
}

function optionalInteger(value: unknown, path: string): void {
  if (value !== null) {
    integer(value, path);
  }
}

function boolean(value: unknown, path: string): void {
  if (typeof value !== "boolean") {
    fail(path, "expected boolean");
  }
}

function assertNoPayload(value: unknown, path: string): void {
  if (Array.isArray(value)) {
    value.forEach((entry, index) => assertNoPayload(entry, `${path}[${index}]`));
    return;
  }
  if (typeof value !== "object" || value === null) {
    return;
  }
  for (const [key, child] of Object.entries(value)) {
    if (key === "payload") {
      fail(`${path}.payload`, "product payload is forbidden");
    }
    assertNoPayload(child, `${path}.${key}`);
  }
}

function fail(path: string, message: string): never {
  throw new HistoryProtocolValidationError(path, message);
}

/**
 * What sits below the page's oldest entry.
 *
 * `baseline` means retention took everything before it, so the position is
 * where the surviving history begins rather than where the document did. A
 * renderer that draws it as the origin claims data the authority discarded,
 * which is why the two are separate variants and not a flag.
 */
function floor(value: unknown, path: string): void {
  const record = object(value, path);
  oneOf(record.kind, `${path}.kind`, HISTORY_PAGE_FLOORS);
  keys(record, path, variantKeys("HistoryPageFloorProjection", record, path));
  if (record.kind === "baseline") {
    integer(record.prunedEntryCount, `${path}.prunedEntryCount`);
  }
}

function variantKeys(
  type: string,
  record: Record<string, unknown>,
  path: string,
): ReadonlyArray<string> {
  return wireVariantFields(
    HISTORY_VARIANT_FIELDS,
    HISTORY_VARIANT_FIELDS_DISCRIMINANTS,
    type,
    record,
    (discriminant) => fail(path, `no generated fields for ${type}.${String(discriminant)}`),
  );
}
