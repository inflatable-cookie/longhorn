import { OPERATION_FIELDS } from "./generated/fields.ts";
import {
  OPERATION_CANCELLATION_OUTCOMES,
  OPERATION_CANCELLATION_STATUSES,
  OPERATION_CHANGED_KINDS,
  OPERATION_EXECUTOR_DISPATCH_KINDS,
  OPERATION_MUTATION_KINDS,
  OPERATION_MUTATION_RECEIPT_KINDS,
  OPERATION_MUTATION_STATUSES,
  OPERATION_PROGRESS_KINDS,
  OPERATION_PROTOCOL_VERSION,
  OPERATION_REJECTION_CODES,
  OPERATION_STATES,
  type OperationCancellationCommand,
  type OperationCancellationResult,
  type OperationChangedEvent,
  type OperationEntryProjection,
  type OperationMutationCommand,
  type OperationMutationResult,
  type OperationSnapshot,
  type OperationSnapshotQuery,
  type OperationSnapshotResponse,
} from "./generated/protocol.ts";

const FORBIDDEN_PAYLOAD_KEYS = new Set([
  "payload",
  "result",
  "artifact",
  "report",
  "log",
]);

export class OperationProtocolValidationError extends Error {
  constructor(readonly path: string, message: string) {
    super(`incompatible operation protocol at ${path}: ${message}`);
    this.name = "OperationProtocolValidationError";
  }
}

export function assertValidOperationSnapshotQuery(
  value: unknown,
): asserts value is OperationSnapshotQuery {
  const object = exactObject(value, "$", OPERATION_FIELDS.OperationSnapshotQuery);
  protocol(object.protocolVersion, "$.protocolVersion");
  text(object.requestId, "$.requestId");
}

export function assertValidOperationSnapshotResponse(
  value: unknown,
): asserts value is OperationSnapshotResponse {
  assertPayloadFree(value);
  const object = exactObject(value, "$", OPERATION_FIELDS.OperationSnapshotResponse);
  text(object.requestId, "$.requestId");
  assertValidOperationSnapshot(object.snapshot);
}

export function assertValidOperationSnapshot(
  value: unknown,
): asserts value is OperationSnapshot {
  assertPayloadFree(value);
  const object = exactObject(value, "$", OPERATION_FIELDS.OperationSnapshot);
  protocol(object.protocolVersion, "$.protocolVersion");
  authority(object.authority, "$.authority");
  natural(object.catalogueRevision, "$.catalogueRevision");
  boolean(object.closed, "$.closed");
  const limits = exactObject(object.limits, "$.limits", OPERATION_FIELDS.OperationCatalogueLimitsProjection);
  natural(limits.maximumActiveOperations, "$.limits.maximumActiveOperations");
  natural(limits.maximumTerminalOperations, "$.limits.maximumTerminalOperations");
  natural(limits.maximumTerminalEncodedWeight, "$.limits.maximumTerminalEncodedWeight");
  natural(object.terminalEvictionCount, "$.terminalEvictionCount");
  natural(object.retainedTerminalEncodedWeight, "$.retainedTerminalEncodedWeight");
  array(object.active, "$.active").forEach((entry, index) =>
    operationEntry(entry, `$.active[${index}]`)
  );
  array(object.recent, "$.recent").forEach((entry, index) =>
    operationEntry(entry, `$.recent[${index}]`)
  );
}

export function assertValidOperationMutationCommand(
  value: unknown,
): asserts value is OperationMutationCommand {
  assertPayloadFree(value);
  const object = record(value, "$");
  member(object.kind, OPERATION_MUTATION_KINDS, "$.kind");
  protocol(object.protocolVersion, "$.protocolVersion");
  text(object.requestId, "$.requestId");
  authority(object.authority, "$.authority");
  switch (object.kind) {
    case "register":
      exactKeys(object, "$", ["kind", "requestId", "protocolVersion", "authority", "expectedCatalogueRevision", "operationId", "kindId", "scopeId", "label", "initialState", "cancellationSupport", "retryOf"]);
      natural(object.expectedCatalogueRevision, "$.expectedCatalogueRevision");
      text(object.operationId, "$.operationId");
      text(object.kindId, "$.kindId");
      nullableText(object.scopeId, "$.scopeId");
      text(object.label, "$.label");
      member(object.initialState, OPERATION_STATES, "$.initialState");
      member(object.cancellationSupport, ["supported", "unsupported"] as const, "$.cancellationSupport");
      nullableText(object.retryOf, "$.retryOf");
      break;
    case "progress":
      exactKeys(object, "$", ["kind", "requestId", "protocolVersion", "authority", "operationId", "expectedOperationRevision", "overall", "phase"]);
      operationTarget(object);
      progressOverall(object.overall, "$.overall");
      if (object.phase !== null) phase(object.phase, "$.phase");
      break;
    case "transition":
      exactKeys(object, "$", ["kind", "requestId", "protocolVersion", "authority", "operationId", "expectedOperationRevision", "nextState"]);
      operationTarget(object);
      member(object.nextState, OPERATION_STATES, "$.nextState");
      break;
    case "changeRetention":
      exactKeys(object, "$", ["kind", "requestId", "protocolVersion", "authority", "expectedCatalogueRevision", "limits"]);
      natural(object.expectedCatalogueRevision, "$.expectedCatalogueRevision");
      assertValidOperationSnapshot({
        protocolVersion: 1, authority: object.authority, catalogueRevision: 0,
        closed: false, limits: object.limits, terminalEvictionCount: 0,
        retainedTerminalEncodedWeight: 0, active: [], recent: [],
      });
      break;
    case "dismiss":
      exactKeys(object, "$", ["kind", "requestId", "protocolVersion", "authority", "operationId", "expectedOperationRevision"]);
      operationTarget(object);
      break;
    case "teardown":
      exactKeys(object, "$", ["kind", "requestId", "protocolVersion", "authority", "expectedCatalogueRevision", "resolutions"]);
      natural(object.expectedCatalogueRevision, "$.expectedCatalogueRevision");
      array(object.resolutions, "$.resolutions").forEach((resolution, index) => teardownResolution(resolution, `$.resolutions[${index}]`));
      break;
  }
}

export function assertValidOperationCancellationCommand(
  value: unknown,
): asserts value is OperationCancellationCommand {
  assertPayloadFree(value);
  const object = exactObject(value, "$", OPERATION_FIELDS.OperationCancellationCommand);
  text(object.requestId, "$.requestId");
  protocol(object.protocolVersion, "$.protocolVersion");
  authority(object.authority, "$.authority");
  operationTarget(object);
}

export function assertValidOperationMutationResult(
  value: unknown,
): asserts value is OperationMutationResult {
  resultBase(value, OPERATION_MUTATION_STATUSES);
  const object = value as Record<string, unknown>;
  if (object.status === "committed") {
    exactKeys(object, "$", ["status", "requestId", "snapshot", "receipt"]);
    mutationReceipt(object.receipt, "$.receipt");
  } else {
    exactKeys(object, "$", ["status", "requestId", "snapshot", "rejection"]);
    rejection(object.rejection, "$.rejection");
  }
}

export function assertValidOperationCancellationResult(
  value: unknown,
): asserts value is OperationCancellationResult {
  resultBase(value, OPERATION_CANCELLATION_STATUSES);
  const object = value as Record<string, unknown>;
  if (object.status === "committed") {
    exactKeys(object, "$", ["status", "requestId", "snapshot", "receipt", "executorDispatch"]);
    cancellationReceipt(object.receipt, "$.receipt");
    executorDispatch(object.executorDispatch, "$.executorDispatch");
  } else {
    exactKeys(object, "$", ["status", "requestId", "snapshot", "rejection"]);
    rejection(object.rejection, "$.rejection");
  }
}

export function assertValidOperationChangedEvent(
  value: unknown,
): asserts value is OperationChangedEvent {
  assertPayloadFree(value);
  const object = exactObject(value, "$", OPERATION_FIELDS.OperationChangedEvent);
  protocol(object.protocolVersion, "$.protocolVersion");
  text(object.requestId, "$.requestId");
  authority(object.authority, "$.authority");
  natural(object.previousCatalogueRevision, "$.previousCatalogueRevision");
  natural(object.committedCatalogueRevision, "$.committedCatalogueRevision");
  nullableText(object.operationId, "$.operationId");
  member(object.kind, OPERATION_CHANGED_KINDS, "$.kind");
}

export function assertPayloadFree(value: unknown): void {
  visit(value, "$", new Set());
}

function resultBase(value: unknown, statuses: readonly string[]): void {
  assertPayloadFree(value);
  const object = record(value, "$");
  member(object.status, statuses, "$.status");
  text(object.requestId, "$.requestId");
  assertValidOperationSnapshot(object.snapshot);
}

function operationEntry(value: unknown, path: string): asserts value is OperationEntryProjection {
  const object = exactObject(value, path, OPERATION_FIELDS.OperationEntryProjection);
  authority(object.authority, `${path}.authority`);
  text(object.operationId, `${path}.operationId`);
  text(object.kindId, `${path}.kindId`);
  nullableText(object.scopeId, `${path}.scopeId`);
  text(object.label, `${path}.label`);
  member(object.cancellationSupport, ["supported", "unsupported"] as const, `${path}.cancellationSupport`);
  nullableText(object.retryOf, `${path}.retryOf`);
  natural(object.sequence, `${path}.sequence`);
  natural(object.revision, `${path}.revision`);
  natural(object.lastChangedCatalogueRevision, `${path}.lastChangedCatalogueRevision`);
  member(object.state, OPERATION_STATES, `${path}.state`);
  const progress = exactObject(object.progress, `${path}.progress`, OPERATION_FIELDS.OperationProgressProjection);
  natural(progress.sequence, `${path}.progress.sequence`);
  progressOverall(progress.overall, `${path}.progress.overall`);
  if (progress.phase !== null) phase(progress.phase, `${path}.progress.phase`);
  natural(object.encodedMetadataWeight, `${path}.encodedMetadataWeight`);
}

function progressOverall(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, OPERATION_PROGRESS_KINDS, `${path}.kind`);
  if (object.kind === "indeterminate") exactKeys(object, path, ["kind"]);
  if (object.kind === "units") {
    exactKeys(object, path, ["kind", "completed", "total"]);
    finite(object.completed, `${path}.completed`);
    finite(object.total, `${path}.total`);
  }
  if (object.kind === "normalized") {
    exactKeys(object, path, ["kind", "value"]);
    finite(object.value, `${path}.value`);
  }
}

function phase(value: unknown, path: string): void {
  const object = exactObject(value, path, OPERATION_FIELDS.OperationPhaseProgressProjection);
  text(object.phaseId, `${path}.phaseId`);
  text(object.label, `${path}.label`);
  finite(object.completed, `${path}.completed`);
  finite(object.total, `${path}.total`);
}

function teardownResolution(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, ["complete", "transfer"] as const, `${path}.kind`);
  if (object.kind === "complete") {
    exactKeys(object, path, ["kind", "operationId", "expectedOperationRevision", "state"]);
    member(object.state, OPERATION_STATES, `${path}.state`);
  } else {
    exactKeys(object, path, ["kind", "operationId", "expectedOperationRevision", "targetAuthority"]);
    authority(object.targetAuthority, `${path}.targetAuthority`);
  }
  text(object.operationId, `${path}.operationId`);
  natural(object.expectedOperationRevision, `${path}.expectedOperationRevision`);
}

function rejection(value: unknown, path: string): void {
  const object = exactObject(value, path, OPERATION_FIELDS.OperationRejection);
  member(object.code, OPERATION_REJECTION_CODES, `${path}.code`);
  text(object.detail, `${path}.detail`);
  boolean(object.refreshRequired, `${path}.refreshRequired`);
}

function mutationReceipt(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, OPERATION_MUTATION_RECEIPT_KINDS, `${path}.kind`);
  switch (object.kind) {
    case "registered":
      exactKeys(object, path, ["kind", "operation", "previousCatalogueRevision", "committedCatalogueRevision"]);
      operationEntry(object.operation, `${path}.operation`);
      catalogueRevisionPair(object, path);
      break;
    case "progressed":
      exactKeys(object, path, ["kind", "operationId", "previousOperationRevision", "committedOperationRevision", "previousProgressSequence", "committedProgress", "previousCatalogueRevision", "committedCatalogueRevision"]);
      operationRevisionPair(object, path);
      natural(object.previousProgressSequence, `${path}.previousProgressSequence`);
      const progress = exactObject(object.committedProgress, `${path}.committedProgress`, OPERATION_FIELDS.OperationProgressProjection);
      natural(progress.sequence, `${path}.committedProgress.sequence`);
      progressOverall(progress.overall, `${path}.committedProgress.overall`);
      if (progress.phase !== null) phase(progress.phase, `${path}.committedProgress.phase`);
      catalogueRevisionPair(object, path);
      break;
    case "transitioned":
      exactKeys(object, path, ["kind", "operationId", "previousState", "committedState", "previousOperationRevision", "committedOperationRevision", "previousCatalogueRevision", "committedCatalogueRevision", "evicted"]);
      member(object.previousState, OPERATION_STATES, `${path}.previousState`);
      member(object.committedState, OPERATION_STATES, `${path}.committedState`);
      operationRevisionPair(object, path);
      catalogueRevisionPair(object, path);
      removals(object.evicted, `${path}.evicted`);
      break;
    case "retentionChanged":
      exactKeys(object, path, ["kind", "previousLimits", "committedLimits", "previousCatalogueRevision", "committedCatalogueRevision", "evicted", "retainedTerminalEncodedWeight"]);
      limits(object.previousLimits, `${path}.previousLimits`);
      limits(object.committedLimits, `${path}.committedLimits`);
      catalogueRevisionPair(object, path);
      removals(object.evicted, `${path}.evicted`);
      natural(object.retainedTerminalEncodedWeight, `${path}.retainedTerminalEncodedWeight`);
      break;
    case "dismissed":
      exactKeys(object, path, ["kind", "removed", "previousCatalogueRevision", "committedCatalogueRevision"]);
      removal(object.removed, `${path}.removed`);
      catalogueRevisionPair(object, path);
      break;
    case "tornDown":
      exactKeys(object, path, ["kind", "previousCatalogueRevision", "committedCatalogueRevision", "outcomes", "evicted"]);
      catalogueRevisionPair(object, path);
      array(object.outcomes, `${path}.outcomes`).forEach((outcome, index) => teardownOutcome(outcome, `${path}.outcomes[${index}]`));
      removals(object.evicted, `${path}.evicted`);
      break;
  }
}

function cancellationReceipt(value: unknown, path: string): void {
  const object = exactObject(value, path, OPERATION_FIELDS.OperationCancellationReceiptProjection);
  text(object.operationId, `${path}.operationId`);
  member(object.outcome, OPERATION_CANCELLATION_OUTCOMES, `${path}.outcome`);
  member(object.previousState, OPERATION_STATES, `${path}.previousState`);
  member(object.committedState, OPERATION_STATES, `${path}.committedState`);
  operationRevisionPair(object, path);
  catalogueRevisionPair(object, path);
  removals(object.evicted, `${path}.evicted`);
}

function executorDispatch(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, OPERATION_EXECUTOR_DISPATCH_KINDS, `${path}.kind`);
  if (object.kind === "failed") {
    exactKeys(object, path, ["kind", "code", "message", "retryable"]);
    text(object.code, `${path}.code`);
    text(object.message, `${path}.message`);
    boolean(object.retryable, `${path}.retryable`);
  } else {
    exactKeys(object, path, ["kind"]);
  }
}

function removal(value: unknown, path: string): void {
  const object = exactObject(value, path, OPERATION_FIELDS.OperationRemovalProjection);
  text(object.operationId, `${path}.operationId`);
  natural(object.sequence, `${path}.sequence`);
  natural(object.encodedWeight, `${path}.encodedWeight`);
  member(object.reason, ["evicted", "dismissed"] as const, `${path}.reason`);
}

function removals(value: unknown, path: string): void {
  array(value, path).forEach((entry, index) => removal(entry, `${path}[${index}]`));
}

function teardownOutcome(value: unknown, path: string): void {
  const object = record(value, path);
  member(object.kind, ["completed", "transferred"] as const, `${path}.kind`);
  if (object.kind === "completed") {
    exactKeys(object, path, ["kind", "operationId", "state", "previousOperationRevision", "committedOperationRevision"]);
    member(object.state, OPERATION_STATES, `${path}.state`);
  } else {
    exactKeys(object, path, ["kind", "operationId", "previousOperationRevision", "targetAuthority"]);
    authority(object.targetAuthority, `${path}.targetAuthority`);
  }
  text(object.operationId, `${path}.operationId`);
  natural(object.previousOperationRevision, `${path}.previousOperationRevision`);
  if (object.kind === "completed") natural(object.committedOperationRevision, `${path}.committedOperationRevision`);
}

function limits(value: unknown, path: string): void {
  const object = exactObject(value, path, OPERATION_FIELDS.OperationCatalogueLimitsProjection);
  natural(object.maximumActiveOperations, `${path}.maximumActiveOperations`);
  natural(object.maximumTerminalOperations, `${path}.maximumTerminalOperations`);
  natural(object.maximumTerminalEncodedWeight, `${path}.maximumTerminalEncodedWeight`);
}

function operationRevisionPair(object: Record<string, unknown>, path: string): void {
  text(object.operationId, `${path}.operationId`);
  natural(object.previousOperationRevision, `${path}.previousOperationRevision`);
  natural(object.committedOperationRevision, `${path}.committedOperationRevision`);
}

function catalogueRevisionPair(object: Record<string, unknown>, path: string): void {
  natural(object.previousCatalogueRevision, `${path}.previousCatalogueRevision`);
  natural(object.committedCatalogueRevision, `${path}.committedCatalogueRevision`);
}

function operationTarget(object: Record<string, unknown>): void {
  text(object.operationId, "$.operationId");
  natural(object.expectedOperationRevision, "$.expectedOperationRevision");
}

function authority(value: unknown, path: string): void {
  const object = exactObject(value, path, OPERATION_FIELDS.OperationAuthorityProjection);
  text(object.authorityId, `${path}.authorityId`);
  natural(object.authorityEpoch, `${path}.authorityEpoch`);
  if (object.authorityEpoch === 0) fail(`${path}.authorityEpoch`, "must be nonzero");
}

function protocol(value: unknown, path: string): void {
  natural(value, path);
  if (value !== OPERATION_PROTOCOL_VERSION) fail(path, `expected ${OPERATION_PROTOCOL_VERSION}`);
}

function exactObject(value: unknown, path: string, keys: readonly string[]): Record<string, unknown> {
  const object = record(value, path);
  exactKeys(object, path, keys);
  return object;
}

function exactKeys(object: Record<string, unknown>, path: string, keys: readonly string[]): void {
  const actual = Object.keys(object).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    fail(path, `expected keys ${expected.join(",")}; received ${actual.join(",")}`);
  }
}

function record(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) fail(path, "expected object");
  return value as Record<string, unknown>;
}

function array(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) fail(path, "expected array");
  return value;
}

function text(value: unknown, path: string): asserts value is string {
  if (typeof value !== "string" || value.length === 0) fail(path, "expected non-empty string");
}

function nullableText(value: unknown, path: string): void {
  if (value !== null) text(value, path);
}

function natural(value: unknown, path: string): asserts value is number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) fail(path, "expected non-negative safe integer");
}

function finite(value: unknown, path: string): asserts value is number {
  if (typeof value !== "number" || !Number.isFinite(value)) fail(path, "expected finite number");
}

function boolean(value: unknown, path: string): asserts value is boolean {
  if (typeof value !== "boolean") fail(path, "expected boolean");
}

function member(value: unknown, values: readonly string[], path: string): asserts value is string {
  if (typeof value !== "string" || !values.includes(value)) fail(path, `expected one of ${values.join(",")}`);
}

function visit(value: unknown, path: string, seen: Set<object>): void {
  if (typeof value !== "object" || value === null) return;
  if (seen.has(value)) fail(path, "cyclic value");
  seen.add(value);
  if (Array.isArray(value)) {
    value.forEach((entry, index) => visit(entry, `${path}[${index}]`, seen));
  } else {
    for (const [key, entry] of Object.entries(value)) {
      if (FORBIDDEN_PAYLOAD_KEYS.has(key)) fail(`${path}.${key}`, "product payload field is forbidden");
      visit(entry, `${path}.${key}`, seen);
    }
  }
  seen.delete(value);
}

function fail(path: string, message: string): never {
  throw new OperationProtocolValidationError(path, message);
}
