import { isOpaqueId, matchesDomainIdRule } from "../core/identifiers.ts";
import {
  checkWireFields,
  readWireRecord,
  unsigned32,
  unsignedInteger,
  wireVariantFields,
} from "../core/record.ts";
import { BRIDGE_DOMAIN_ID_RULE } from "../bridge/generated/protocol.ts";
import {
  TRANSFER_VARIANT_FIELDS,
  TRANSFER_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";
import {
  PANEL_TRANSFER_ERROR_CODES,
  PANEL_TRANSFER_RESPONSE_STATUSES,
  TRANSFER_ABORT_DOMAINS,
  TRANSFER_CANCEL_RESPONSE_STATUSES,
  TRANSFER_COMMIT_SELECTOR_KINDS,
  TRANSFER_ERROR_CODES,
  TRANSFER_LEASE_RESPONSE_STATUSES,
  TRANSFER_PANEL_HOST_BINDING_KINDS,
  TRANSFER_PROTOCOL_VERSION,
  TRANSFER_SCREEN_POINT_FIELDS,
  TRANSFER_SESSION_CANCELLATION_STATUSES,
  TRANSFER_SESSION_RESPONSE_STATUSES,
  TRANSFER_TARGET_BINDING_KINDS,
  TRANSFER_TARGET_RESOLUTION_PATHS,
} from "./generated/protocol.ts";
import type {
  PanelTransferResponse,
  TransferAbort,
  TransferCancelResponse,
  TransferClientSnapshot,
  TransferCommitSelector,
  TransferCommittedTarget,
  TransferLeaseResponse,
  TransferSessionResponse,
  TransferSessionStarted,
  TransferTargetBinding,
} from "./generated/protocol.ts";
import { isTransferSessionId } from "./payload.ts";
import { TRANSFER_FIELDS } from "./generated/fields.ts";
import { assertValidLayoutMutationRejectionCode } from "@inflatable-cookie/longhorn/layout";
import { assertValidSurfaceDocument } from "@inflatable-cookie/longhorn/surfaces";

export type TransferProtocolValidationCode =
  | "unsupported_protocol_version"
  | "unknown_target_binding"
  | "unknown_commit_selector"
  | "unknown_abort_domain"
  | "unknown_transfer_error_code"
  | "unknown_panel_error_code"
  | "unknown_response_status"
  | "invalid_client_snapshot"
  | "invalid_object"
  | "unknown_field"
  | "missing_field";

export class TransferProtocolValidationError extends Error {
  readonly code: TransferProtocolValidationCode;
  readonly received: unknown;

  constructor(code: TransferProtocolValidationCode, received: unknown) {
    super(`incompatible transfer protocol: ${code}`);
    this.name = "TransferProtocolValidationError";
    this.code = code;
    this.received = received;
  }
}

export function assertTransferProtocolVersion(
  version: unknown,
): asserts version is typeof TRANSFER_PROTOCOL_VERSION {
  if (version !== TRANSFER_PROTOCOL_VERSION) {
    throw new TransferProtocolValidationError("unsupported_protocol_version", version);
  }
}

export function assertValidTransferTargetBinding(
  value: unknown,
): asserts value is TransferTargetBinding {
  const candidate = record(value);
  assertKnown(candidate.kind, TRANSFER_TARGET_BINDING_KINDS, "unknown_target_binding");
  record(candidate, variantKeys("TransferTargetBinding", candidate));
  assertFields(candidate, {
    host_binding_id: isOpaqueId,
    document_id: isDomainId,
    revision: unsignedInteger,
  });
  if (candidate.kind === "panel_region") {
    assertFields(candidate, { surface_id: isOpaqueId, region_id: isOpaqueId });
  }
}

export function assertValidTransferClientSnapshot(
  value: unknown,
): asserts value is TransferClientSnapshot {
  const snapshot = record(value, TRANSFER_FIELDS.TransferClientSnapshot);
  assertTransferProtocolVersion(snapshot.protocol_version);
  assertFields(
    snapshot,
    { client_id: isOpaqueId, client_epoch: unsignedInteger },
    "invalid_client_snapshot",
  );
}

export function assertValidTransferCommitSelector(
  value: unknown,
): asserts value is TransferCommitSelector {
  const candidate = record(value);
  assertKnown(candidate.kind, TRANSFER_COMMIT_SELECTOR_KINDS, "unknown_commit_selector");
  record(candidate, variantKeys("TransferCommitSelector", candidate));
  if (candidate.kind === "explicit_zone") {
    assertFields(candidate, { drop_zone_id: isOpaqueId });
  } else {
    assertValidTransferScreenPoint(candidate.point);
  }
}

/** A Rust `ScreenPoint`: integral screen-DIP coordinates. */
export function assertValidTransferScreenPoint(
  value: unknown,
): asserts value is { x: number; y: number } {
  assertFields(record(value, TRANSFER_SCREEN_POINT_FIELDS), { x: isSigned32, y: isSigned32 });
}

export function assertValidTransferAbort(value: unknown): asserts value is TransferAbort {
  const abort = record(value, TRANSFER_FIELDS.TransferAbort);
  assertTransferProtocolVersion(abort.protocol_version);
  assertFields(abort, {
    request_id: isOpaqueId,
    message: isString,
    retryable: isBoolean,
    session_consumed: isBoolean,
  });
  const source = record(abort.source);
  assertKnown(source.domain, TRANSFER_ABORT_DOMAINS, "unknown_abort_domain");
  record(source, variantKeys("TransferAbortSource", source));
  if (source.domain === "transfer") {
    assertKnown(source.code, TRANSFER_ERROR_CODES, "unknown_transfer_error_code");
  } else {
    assertKnown(source.code, PANEL_TRANSFER_ERROR_CODES, "unknown_panel_error_code");
  }
  if (abort.layout_code !== null) {
    assertValidLayoutMutationRejectionCode(abort.layout_code);
  }
}

export function assertValidTransferSessionStarted(
  value: unknown,
): asserts value is TransferSessionStarted {
  const session = record(value, TRANSFER_FIELDS.TransferSessionStarted);
  assertTransferProtocolVersion(session.protocol_version);
  const payload = record(session.payload, TRANSFER_FIELDS.TransferPayload);
  assertTransferProtocolVersion(payload.protocol_version);
  assertFields(session, { request_id: isOpaqueId });
  assertFields(payload, { session_id: isTransferSessionId });
}

export function assertValidTransferCommittedTarget(
  value: unknown,
): asserts value is TransferCommittedTarget {
  const target = record(value, TRANSFER_FIELDS.TransferCommittedTarget);
  assertKnown(target.path, TRANSFER_TARGET_RESOLUTION_PATHS, "invalid_object");
  assertFields(target, {
    window_id: isOpaqueId,
    drop_zone_id: isOpaqueId,
    insertion_position: isNullableUnsigned32,
  });
  assertValidTransferTargetBinding(target.binding);
}

export function assertValidTransferSessionResponse(
  value: unknown,
): asserts value is TransferSessionResponse {
  const response = responseWithStatus(
    value,
    TRANSFER_SESSION_RESPONSE_STATUSES,
    "TransferSessionResponse",
  );
  if (response.status === "started") {
    assertValidTransferSessionStarted(response.session);
  } else {
    assertValidTransferAbort(response.abort);
  }
}

export function assertValidTransferLeaseResponse(
  value: unknown,
): asserts value is TransferLeaseResponse {
  const response = responseWithStatus(
    value,
    TRANSFER_LEASE_RESPONSE_STATUSES,
    "TransferLeaseResponse",
  );
  if (response.status === "published") {
    const lease = record(response.lease, TRANSFER_FIELDS.TransferLeaseReceipt);
    assertTransferProtocolVersion(lease.protocol_version);
    assertFields(lease, {
      request_id: isOpaqueId,
      client_id: isOpaqueId,
      client_epoch: unsignedInteger,
      generation: unsignedInteger,
      zone_count: unsignedInteger,
    });
  } else {
    assertValidTransferAbort(response.abort);
  }
}

export function assertValidTransferCancelResponse(
  value: unknown,
): asserts value is TransferCancelResponse {
  const response = responseWithStatus(
    value,
    TRANSFER_CANCEL_RESPONSE_STATUSES,
    "TransferCancelResponse",
  );
  if (response.status === "cancelled") {
    const cancellation = record(response.cancellation, TRANSFER_FIELDS.TransferCancelReceipt);
    assertTransferProtocolVersion(cancellation.protocol_version);
    assertFields(cancellation, { request_id: isOpaqueId, session_id: isTransferSessionId });
    assertKnown(cancellation.status, TRANSFER_SESSION_CANCELLATION_STATUSES, "invalid_object");
  } else {
    assertValidTransferAbort(response.abort);
  }
}

export function assertValidPanelTransferResponse(
  value: unknown,
): asserts value is PanelTransferResponse {
  const response = responseWithStatus(
    value,
    PANEL_TRANSFER_RESPONSE_STATUSES,
    "PanelTransferResponse",
  );
  if (response.status === "committed") {
    assertPanelTransferCompletion(response.completion);
  } else {
    assertValidTransferAbort(response.abort);
  }
}

function assertPanelTransferCompletion(value: unknown): void {
  const completion = record(value, TRANSFER_FIELDS.PanelTransferCompletion);
  assertTransferProtocolVersion(completion.protocol_version);
  assertFields(completion, {
    request_id: isOpaqueId,
    session_id: isTransferSessionId,
    source_host_binding_id: isOpaqueId,
    target_host_binding_id: isOpaqueId,
    previous_revision: unsignedInteger,
    committed_revision: unsignedInteger,
  });
  assertKnown(completion.source_binding_kind, TRANSFER_PANEL_HOST_BINDING_KINDS, "invalid_object");
  assertKnown(completion.target_binding_kind, TRANSFER_PANEL_HOST_BINDING_KINDS, "invalid_object");
  assertValidSurfaceDocument(completion.authoritative_document);
  assertValidTransferCommittedTarget(completion.target);
}

function responseWithStatus(
  value: unknown,
  statuses: ReadonlyArray<string>,
  type: string,
): Record<string, unknown> {
  const response = record(value);
  assertKnown(response.status, statuses, "unknown_response_status");
  record(response, variantKeys(type, response));
  return response;
}

function variantKeys(type: string, value: Record<string, unknown>): ReadonlyArray<string> {
  return wireVariantFields(
    TRANSFER_VARIANT_FIELDS,
    TRANSFER_VARIANT_FIELDS_DISCRIMINANTS,
    type,
    value,
    (discriminant) => {
      throw new TransferProtocolValidationError("unknown_response_status", { type, discriminant });
    },
  );
}

function assertKnown(
  value: unknown,
  known: ReadonlyArray<string>,
  code: TransferProtocolValidationCode,
): asserts value is string {
  if (typeof value !== "string" || !known.includes(value)) {
    throw new TransferProtocolValidationError(code, value);
  }
}

function assertFields(
  value: Record<string, unknown>,
  checks: Readonly<Record<string, (member: unknown) => boolean>>,
  code: TransferProtocolValidationCode = "invalid_object",
): void {
  for (const [key, admits] of Object.entries(checks)) {
    if (!admits(value[key])) {
      throw new TransferProtocolValidationError(code, { key, value });
    }
  }
}

function isString(value: unknown): boolean {
  return typeof value === "string";
}

function isBoolean(value: unknown): boolean {
  return typeof value === "boolean";
}

function isDomainId(value: unknown): boolean {
  return matchesDomainIdRule(value, BRIDGE_DOMAIN_ID_RULE);
}

function isSigned32(value: unknown): boolean {
  const limit = 0x80_00_00_00;
  return Number.isInteger(value) && Number(value) >= -limit && Number(value) < limit;
}

function isNullableUnsigned32(value: unknown): boolean {
  return value === null || unsigned32(value);
}

function record(value: unknown, allowed?: ReadonlyArray<string>): Record<string, unknown> {
  const result = readWireRecord(value, () => {
    throw new TransferProtocolValidationError("invalid_object", value);
  });
  checkWireFields(result, allowed, (code, key) => {
    throw new TransferProtocolValidationError(code, { key, value });
  });
  return result;
}
