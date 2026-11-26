import {
  checkWireFields,
  readWireRecord,
  unsignedInteger,
  wireVariantFields,
} from "../core/record.ts";
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
  TRANSFER_PROTOCOL_VERSION,
  TRANSFER_SESSION_CANCELLATION_STATUSES,
  TRANSFER_SESSION_RESPONSE_STATUSES,
  TRANSFER_TARGET_BINDING_KINDS,
} from "./generated/protocol.ts";
import type {
  PanelTransferResponse,
  TransferAbort,
  TransferCancelResponse,
  TransferClientSnapshot,
  TransferCommitSelector,
  TransferLeaseResponse,
  TransferSessionResponse,
  TransferTargetBinding,
} from "./generated/protocol.ts";
import { isTransferSessionId } from "./payload.ts";
import { TRANSFER_FIELDS } from "./generated/fields.ts";
import { assertValidLayoutMutationRejectionCode } from "@inflatable-cookie/longhorn/layout";

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
}

export function assertValidTransferClientSnapshot(
  value: unknown,
): asserts value is TransferClientSnapshot {
  const snapshot = record(value, TRANSFER_FIELDS.TransferClientSnapshot);
  assertTransferProtocolVersion(snapshot.protocol_version);
  if (typeof snapshot.client_id !== "string" || !unsignedInteger(snapshot.client_epoch)) {
    throw new TransferProtocolValidationError("invalid_client_snapshot", value);
  }
}

export function assertValidTransferCommitSelector(
  value: unknown,
): asserts value is TransferCommitSelector {
  const candidate = record(value);
  assertKnown(candidate.kind, TRANSFER_COMMIT_SELECTOR_KINDS, "unknown_commit_selector");
  record(candidate, variantKeys("TransferCommitSelector", candidate));
}

export function assertValidTransferAbort(value: unknown): asserts value is TransferAbort {
  const abort = record(value, TRANSFER_FIELDS.TransferAbort);
  assertTransferProtocolVersion(abort.protocol_version);
  if (
    typeof abort.request_id !== "string" ||
    typeof abort.message !== "string" ||
    typeof abort.retryable !== "boolean" ||
    typeof abort.session_consumed !== "boolean"
  ) {
    throw new TransferProtocolValidationError("invalid_object", abort);
  }
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

export function assertValidTransferSessionResponse(
  value: unknown,
): asserts value is TransferSessionResponse {
  const response = responseWithStatus(
    value,
    TRANSFER_SESSION_RESPONSE_STATUSES,
    "TransferSessionResponse",
  );
  if (response.status === "started") {
    const session = record(response.session, TRANSFER_FIELDS.TransferSessionStarted);
    assertTransferProtocolVersion(session.protocol_version);
    const payload = record(session.payload, TRANSFER_FIELDS.TransferPayload);
    assertTransferProtocolVersion(payload.protocol_version);
    if (typeof session.request_id !== "string" || !isTransferSessionId(payload.session_id)) {
      throw new TransferProtocolValidationError("invalid_object", session);
    }
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
    if (
      typeof lease.request_id !== "string" ||
      typeof lease.client_id !== "string" ||
      !unsignedInteger(lease.client_epoch) ||
      !unsignedInteger(lease.generation) ||
      !unsignedInteger(lease.zone_count)
    ) {
      throw new TransferProtocolValidationError("invalid_object", lease);
    }
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
    if (
      typeof cancellation.request_id !== "string" ||
      !isTransferSessionId(cancellation.session_id)
    ) {
      throw new TransferProtocolValidationError("invalid_object", cancellation);
    }
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
    const completion = record(response.completion, TRANSFER_FIELDS.PanelTransferCompletion);
    assertTransferProtocolVersion(completion.protocol_version);
    // oxlint-disable-next-line quality/no-known-value-widening -- Runtime validation also covers structurally typed callers and untrusted wire values.
    assertValidTransferTargetBinding(record(record(completion.target).binding));
  } else {
    assertValidTransferAbort(response.abort);
  }
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

function record(value: unknown, allowed?: ReadonlyArray<string>): Record<string, unknown> {
  const result = readWireRecord(value, () => {
    throw new TransferProtocolValidationError("invalid_object", value);
  });
  checkWireFields(result, allowed, (code, key) => {
    throw new TransferProtocolValidationError(code, { key, value });
  });
  return result;
}
