import {
  NATIVE_CONTENT_CONNECT_STATUSES,
  NATIVE_CONTENT_DECISION_STATUSES,
  NATIVE_CONTENT_FAILURE_PHASES,
  NATIVE_CONTENT_REJECTION_CODES,
  NATIVE_CONTENT_RETRY_CLASSES,
  NATIVE_CONTENT_SNAPSHOT_STATUSES,
  NATIVE_CONTENT_UPDATE_STATUSES,
  type NativeContentConnectRequest,
  type NativeContentConnectResult,
  type NativeContentContentSizeDecisionRequest,
  type NativeContentContentSizeDecisionResult,
  type NativeContentDesiredUpdateRequest,
  type NativeContentDesiredUpdateResult,
  type NativeContentSnapshotRequest,
  type NativeContentSnapshotResult,
} from "../generated/protocol.ts";
import {
  assertNativeContentProtocolVersion,
  assertProductPayloadFree,
  exactKeys,
  exactObject,
  fail,
  member,
  natural,
  opaqueId,
  positive,
  record,
  text,
} from "./base.ts";
import {
  assertCompatibleContentSizeDecision,
  assertCompatibleContentSizeProposal,
  assertCompatibleDesiredUpdate,
  assertCompatibleNativeContentChangedEvent,
  assertCompatibleNativeContentSnapshot,
  desiredReceipt,
  proposalReceipt,
} from "./state.ts";

export function assertCompatibleNativeContentConnectRequest(
  value: unknown,
): asserts value is NativeContentConnectRequest {
  const object = exactObject(value, "$", [
    "protocol_version",
    "request_id",
    "island_id",
  ]);
  requestEnvelope(object);
}

export function assertCompatibleNativeContentSnapshotRequest(
  value: unknown,
): asserts value is NativeContentSnapshotRequest {
  const object = exactObject(value, "$", [
    "protocol_version",
    "request_id",
    "island_id",
    "client_epoch",
  ]);
  requestEnvelope(object);
  positive(object.client_epoch, "$.client_epoch");
}

export function assertCompatibleNativeContentDesiredUpdateRequest(
  value: unknown,
): asserts value is NativeContentDesiredUpdateRequest {
  assertProductPayloadFree(value);
  const object = exactObject(value, "$", [
    "protocol_version",
    "request_id",
    "island_id",
    "client_epoch",
    "expected_desired_revision",
    "update",
  ]);
  requestEnvelope(object);
  positive(object.client_epoch, "$.client_epoch");
  natural(object.expected_desired_revision, "$.expected_desired_revision");
  assertCompatibleDesiredUpdate(object.update);
}

export function assertCompatibleNativeContentDecisionRequest(
  value: unknown,
): asserts value is NativeContentContentSizeDecisionRequest {
  assertProductPayloadFree(value);
  const object = exactObject(value, "$", [
    "protocol_version",
    "request_id",
    "island_id",
    "client_epoch",
    "proposal",
    "decision",
  ]);
  requestEnvelope(object);
  positive(object.client_epoch, "$.client_epoch");
  assertCompatibleContentSizeProposal(object.proposal);
  assertCompatibleContentSizeDecision(object.decision);
}

export function assertCompatibleNativeContentConnectResult(
  value: unknown,
): asserts value is NativeContentConnectResult {
  assertProductPayloadFree(value);
  const object = record(value, "$");
  member(object.status, NATIVE_CONTENT_CONNECT_STATUSES, "$.status");
  responseCorrelation(object);
  if (object.status === "connected") {
    exactKeys(object, "$", ["status", "request_id", "snapshot"]);
    assertCompatibleNativeContentSnapshot(object.snapshot);
  } else {
    exactKeys(object, "$", ["status", "request_id", "rejection"]);
    rejection(object.rejection, "$.rejection");
  }
}

export function assertCompatibleNativeContentSnapshotResult(
  value: unknown,
): asserts value is NativeContentSnapshotResult {
  assertProductPayloadFree(value);
  const object = record(value, "$");
  member(object.status, NATIVE_CONTENT_SNAPSHOT_STATUSES, "$.status");
  responseCorrelation(object);
  if (object.status === "ready") {
    exactKeys(object, "$", ["status", "request_id", "snapshot"]);
    assertCompatibleNativeContentSnapshot(object.snapshot);
  } else {
    exactKeys(object, "$", ["status", "request_id", "rejection"]);
    rejection(object.rejection, "$.rejection");
  }
}

export function assertCompatibleNativeContentDesiredUpdateResult(
  value: unknown,
): asserts value is NativeContentDesiredUpdateResult {
  assertProductPayloadFree(value);
  const object = record(value, "$");
  member(object.status, NATIVE_CONTENT_UPDATE_STATUSES, "$.status");
  responseCorrelation(object);
  if (object.status === "committed") {
    exactKeys(object, "$", [
      "status",
      "request_id",
      "snapshot",
      "receipt",
      "event",
    ]);
    assertCompatibleNativeContentSnapshot(object.snapshot);
    desiredReceipt(object.receipt, "$.receipt");
    assertCompatibleNativeContentChangedEvent(object.event);
    const event = object.event as Record<string, unknown>;
    const change = event.change as Record<string, unknown>;
    if (
      change.kind !== "desired_updated" ||
      change.request_id !== object.request_id
    ) {
      fail("$.event.change", "desired update event correlation mismatch");
    }
  } else {
    exactKeys(object, "$", ["status", "request_id", "rejection"]);
    rejection(object.rejection, "$.rejection");
  }
}

export function assertCompatibleNativeContentDecisionResult(
  value: unknown,
): asserts value is NativeContentContentSizeDecisionResult {
  assertProductPayloadFree(value);
  const object = record(value, "$");
  member(object.status, NATIVE_CONTENT_DECISION_STATUSES, "$.status");
  responseCorrelation(object);
  if (object.status === "decided") {
    exactKeys(object, "$", [
      "status",
      "request_id",
      "snapshot",
      "receipt",
      "event",
    ]);
    assertCompatibleNativeContentSnapshot(object.snapshot);
    proposalReceipt(object.receipt, "$.receipt");
    assertCompatibleNativeContentChangedEvent(object.event);
    const event = object.event as Record<string, unknown>;
    const change = event.change as Record<string, unknown>;
    if (
      change.kind !== "content_size_decided" ||
      change.request_id !== object.request_id
    ) {
      fail("$.event.change", "content-size decision event correlation mismatch");
    }
  } else {
    exactKeys(object, "$", ["status", "request_id", "rejection"]);
    rejection(object.rejection, "$.rejection");
  }
}

function requestEnvelope(object: Record<string, unknown>): void {
  assertNativeContentProtocolVersion(object.protocol_version);
  opaqueId(object.request_id, "$.request_id");
  opaqueId(object.island_id, "$.island_id");
}

function responseCorrelation(object: Record<string, unknown>): void {
  opaqueId(object.request_id, "$.request_id");
}

function rejection(value: unknown, path: string): void {
  const object = exactObject(value, path, ["code", "message", "phase", "retry"]);
  member(object.code, NATIVE_CONTENT_REJECTION_CODES, `${path}.code`);
  text(object.message, `${path}.message`);
  member(object.phase, NATIVE_CONTENT_FAILURE_PHASES, `${path}.phase`);
  member(object.retry, NATIVE_CONTENT_RETRY_CLASSES, `${path}.retry`);
}
