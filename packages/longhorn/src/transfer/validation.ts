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
  TRANSFER_SESSION_RESPONSE_STATUSES,
  TRANSFER_TARGET_BINDING_KINDS,
  type PanelTransferResponse,
  type TransferAbort,
  type TransferCancelResponse,
  type TransferClientSnapshot,
  type TransferCommitSelector,
  type TransferLeaseResponse,
  type TransferSessionResponse,
  type TransferTargetBinding,
} from "./generated/protocol.ts";
import { TRANSFER_FIELDS } from "./generated/fields.ts";

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
    throw new TransferProtocolValidationError(
      "unsupported_protocol_version",
      version,
    );
  }
}

export function assertValidTransferTargetBinding(
  value: unknown,
): asserts value is TransferTargetBinding {
  const candidate = record(value);
  assertKnown(
    candidate.kind,
    TRANSFER_TARGET_BINDING_KINDS,
    "unknown_target_binding",
  );
  record(candidate, variantKeys("TransferTargetBinding", candidate));
}

export function assertValidTransferClientSnapshot(
  value: unknown,
): asserts value is TransferClientSnapshot {
  const snapshot = record(value, TRANSFER_FIELDS.TransferClientSnapshot);
  assertTransferProtocolVersion(snapshot.protocol_version);
  if (
    typeof snapshot.client_id !== "string" ||
    !unsignedInteger(snapshot.client_epoch) ||
    (snapshot.current_lease_generation !== null &&
      !unsignedInteger(snapshot.current_lease_generation))
  ) {
    throw new TransferProtocolValidationError(
      "invalid_client_snapshot",
      value,
    );
  }
}

export function assertValidTransferCommitSelector(
  value: unknown,
): asserts value is TransferCommitSelector {
  const candidate = record(value);
  assertKnown(
    candidate.kind,
    TRANSFER_COMMIT_SELECTOR_KINDS,
    "unknown_commit_selector",
  );
  record(candidate, variantKeys("TransferCommitSelector", candidate));
}

export function assertValidTransferAbort(
  value: unknown,
): asserts value is TransferAbort {
  const abort = record(value, TRANSFER_FIELDS.TransferAbort);
  assertTransferProtocolVersion(abort.protocol_version);
  const source = record(abort.source);
  assertKnown(
    source.domain,
    TRANSFER_ABORT_DOMAINS,
    "unknown_abort_domain",
  );
  record(source, variantKeys("TransferAbortSource", source));
  if (source.domain === "transfer") {
    assertKnown(
      source.code,
      TRANSFER_ERROR_CODES,
      "unknown_transfer_error_code",
    );
  } else {
    assertKnown(
      source.code,
      PANEL_TRANSFER_ERROR_CODES,
      "unknown_panel_error_code",
    );
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
    assertTransferProtocolVersion(record(session.payload).protocol_version);
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
    assertTransferProtocolVersion(record(response.lease).protocol_version);
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
    assertTransferProtocolVersion(
      record(response.cancellation).protocol_version,
    );
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
    assertValidTransferTargetBinding(
      record(record(completion.target).binding),
    );
  } else {
    assertValidTransferAbort(response.abort);
  }
}

function responseWithStatus(
  value: unknown,
  statuses: readonly string[],
  type: string,
): Record<string, unknown> {
  const response = record(value);
  assertKnown(response.status, statuses, "unknown_response_status");
  record(response, variantKeys(type, response));
  return response;
}

/**
 * Allowed keys for one tagged-union variant, from the generated map, with the
 * discriminant's name read from the map too — this domain tags on `kind`,
 * `status` and `domain`.
 *
 * A missing entry means the generator failed to read the union; every caller
 * runs `assertKnown` over the discriminant above this.
 */
function variantKeys(
  type: string,
  value: Record<string, unknown>,
): readonly string[] {
  const discriminant = value[TRANSFER_VARIANT_FIELDS_DISCRIMINANTS[type] ?? "kind"];
  const keys = TRANSFER_VARIANT_FIELDS[type]?.[discriminant as string];
  if (keys === undefined) {
    throw new TransferProtocolValidationError("unknown_response_status", {
      type,
      discriminant,
    });
  }
  return keys;
}

function assertKnown(
  value: unknown,
  known: readonly string[],
  code: TransferProtocolValidationCode,
): asserts value is string {
  if (typeof value !== "string" || !known.includes(value)) {
    throw new TransferProtocolValidationError(code, value);
  }
}

/**
 * Rejects a non-object, an unknown key, and a missing key.
 *
 * `allowed` comes from the generated field map, so the keys accepted are the
 * Rust struct's and nothing else — contract 010's Boundary Validation Target.
 * Passing no list keeps shape-only behaviour for the tagged unions, whose
 * allowed keys depend on their discriminant and so are not one flat set.
 */
function record(
  value: unknown,
  allowed?: readonly string[],
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TransferProtocolValidationError("invalid_object", value);
  }
  const result = value as Record<string, unknown>;
  if (allowed === undefined) return result;

  const permitted = new Set(allowed);
  for (const key of Object.keys(result)) {
    if (!permitted.has(key)) {
      throw new TransferProtocolValidationError("unknown_field", { key, value });
    }
  }
  for (const key of allowed) {
    if (!(key in result)) {
      throw new TransferProtocolValidationError("missing_field", { key, value });
    }
  }
  return result;
}

function unsignedInteger(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isSafeInteger(value) &&
    value >= 0
  );
}
