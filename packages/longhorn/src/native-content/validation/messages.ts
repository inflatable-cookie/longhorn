import { NATIVE_CONTENT_FIELDS } from "../generated/fields.ts";
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
  variantKeys,
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
  assertValidContentSizeDecision,
  assertValidContentSizeProposal,
  assertValidDesiredUpdate,
  assertValidNativeContentChangedEvent,
  assertValidNativeContentSnapshot,
  desiredReceipt,
  proposalReceipt,
} from "./state.ts";

export function assertValidNativeContentConnectRequest(
  value: unknown,
): asserts value is NativeContentConnectRequest {
  const object = exactObject(value, "$", NATIVE_CONTENT_FIELDS.NativeContentConnectRequest);
  requestEnvelope(object);
}

export function assertValidNativeContentSnapshotRequest(
  value: unknown,
): asserts value is NativeContentSnapshotRequest {
  const object = exactObject(value, "$", NATIVE_CONTENT_FIELDS.NativeContentSnapshotRequest);
  requestEnvelope(object);
  positive(object.client_epoch, "$.client_epoch");
}

export function assertValidNativeContentDesiredUpdateRequest(
  value: unknown,
): asserts value is NativeContentDesiredUpdateRequest {
  assertProductPayloadFree(value);
  const object = exactObject(value, "$", NATIVE_CONTENT_FIELDS.NativeContentDesiredUpdateRequest);
  requestEnvelope(object);
  positive(object.client_epoch, "$.client_epoch");
  natural(object.expected_desired_revision, "$.expected_desired_revision");
  assertValidDesiredUpdate(object.update);
}

export function assertValidNativeContentDecisionRequest(
  value: unknown,
): asserts value is NativeContentContentSizeDecisionRequest {
  assertProductPayloadFree(value);
  const object = exactObject(value, "$", NATIVE_CONTENT_FIELDS.NativeContentContentSizeDecisionRequest);
  requestEnvelope(object);
  positive(object.client_epoch, "$.client_epoch");
  assertValidContentSizeProposal(object.proposal);
  assertValidContentSizeDecision(object.decision);
}

export function assertValidNativeContentConnectResult(
  value: unknown,
): asserts value is NativeContentConnectResult {
  assertProductPayloadFree(value);
  const object = record(value, "$");
  member(object.status, NATIVE_CONTENT_CONNECT_STATUSES, "$.status");
  responseCorrelation(object);
  if (object.status === "connected") {
    exactKeys(object, "$", variantKeys("NativeContentConnectResult", object, "$"));
    assertValidNativeContentSnapshot(object.snapshot);
  } else {
    exactKeys(object, "$", variantKeys("NativeContentConnectResult", object, "$"));
    rejection(object.rejection, "$.rejection");
  }
}

export function assertValidNativeContentSnapshotResult(
  value: unknown,
): asserts value is NativeContentSnapshotResult {
  assertProductPayloadFree(value);
  const object = record(value, "$");
  member(object.status, NATIVE_CONTENT_SNAPSHOT_STATUSES, "$.status");
  responseCorrelation(object);
  if (object.status === "ready") {
    exactKeys(object, "$", variantKeys("NativeContentSnapshotResult", object, "$"));
    assertValidNativeContentSnapshot(object.snapshot);
  } else {
    exactKeys(object, "$", variantKeys("NativeContentSnapshotResult", object, "$"));
    rejection(object.rejection, "$.rejection");
  }
}

export function assertValidNativeContentDesiredUpdateResult(
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
    assertValidNativeContentSnapshot(object.snapshot);
    desiredReceipt(object.receipt, "$.receipt");
    assertValidNativeContentChangedEvent(object.event);
    const event = object.event as Record<string, unknown>;
    const change = event.change as Record<string, unknown>;
    if (
      change.kind !== "desired_updated" ||
      change.request_id !== object.request_id
    ) {
      fail("$.event.change", "desired update event correlation mismatch");
    }
  } else {
    exactKeys(object, "$", variantKeys("NativeContentDesiredUpdateResult", object, "$"));
    rejection(object.rejection, "$.rejection");
  }
}

export function assertValidNativeContentDecisionResult(
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
    assertValidNativeContentSnapshot(object.snapshot);
    proposalReceipt(object.receipt, "$.receipt");
    assertValidNativeContentChangedEvent(object.event);
    const event = object.event as Record<string, unknown>;
    const change = event.change as Record<string, unknown>;
    if (
      change.kind !== "content_size_decided" ||
      change.request_id !== object.request_id
    ) {
      fail("$.event.change", "content-size decision event correlation mismatch");
    }
  } else {
    exactKeys(object, "$", variantKeys("NativeContentContentSizeDecisionResult", object, "$"));
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
  const object = exactObject(value, path, NATIVE_CONTENT_FIELDS.NativeContentProtocolRejection);
  member(object.code, NATIVE_CONTENT_REJECTION_CODES, `${path}.code`);
  text(object.message, `${path}.message`);
  member(object.phase, NATIVE_CONTENT_FAILURE_PHASES, `${path}.phase`);
  member(object.retry, NATIVE_CONTENT_RETRY_CLASSES, `${path}.retry`);
}
