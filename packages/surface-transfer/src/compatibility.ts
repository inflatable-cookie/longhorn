import {
  SURFACE_SESSION_RESPONSE_STATUSES,
  SURFACE_TRANSFER_ABORT_DOMAINS,
  SURFACE_TRANSFER_ERROR_CODES,
  SURFACE_TRANSFER_RESPONSE_STATUSES,
  SURFACE_TRANSFER_TARGET_KINDS,
  type SurfaceSessionResponse,
  type SurfaceTransferAbort,
  type SurfaceTransferResponse,
  type SurfaceTransferTarget,
} from "./generated/protocol.ts";
import {
  TRANSFER_ERROR_CODES,
  assertCompatibleTransferTargetBinding,
  assertTransferProtocolVersion,
} from "@inflatable-cookie/longhorn-transfer";
import {
  assertCompatibleSurfaceMutationRejectionCode,
} from "@inflatable-cookie/longhorn-surfaces";

export type SurfaceTransferProtocolIncompatibilityCode =
  | "unknown_target"
  | "unknown_abort_domain"
  | "unknown_surface_transfer_error_code"
  | "unknown_transfer_error_code"
  | "unknown_response_status";

export class SurfaceTransferProtocolIncompatibilityError extends Error {
  readonly code: SurfaceTransferProtocolIncompatibilityCode;
  readonly received: unknown;

  constructor(
    code: SurfaceTransferProtocolIncompatibilityCode,
    received: unknown,
  ) {
    super(`incompatible Surface transfer protocol: ${code}`);
    this.name = "SurfaceTransferProtocolIncompatibilityError";
    this.code = code;
    this.received = received;
  }
}

export function assertCompatibleSurfaceTransferTarget(
  value: unknown,
): asserts value is SurfaceTransferTarget {
  const target = record(value);
  assertKnown(
    target.kind,
    SURFACE_TRANSFER_TARGET_KINDS,
    "unknown_target",
  );
  if (target.kind === "existing") {
    assertCompatibleTransferTargetBinding(
      record(record(target.target).binding),
    );
  }
}

export function assertCompatibleSurfaceTransferAbort(
  value: unknown,
): asserts value is SurfaceTransferAbort {
  const abort = record(value);
  assertTransferProtocolVersion(abort.protocol_version);
  const source = record(abort.source);
  assertKnown(
    source.domain,
    SURFACE_TRANSFER_ABORT_DOMAINS,
    "unknown_abort_domain",
  );
  if (source.domain === "transfer") {
    assertKnown(
      source.code,
      TRANSFER_ERROR_CODES,
      "unknown_transfer_error_code",
    );
  } else {
    assertKnown(
      source.code,
      SURFACE_TRANSFER_ERROR_CODES,
      "unknown_surface_transfer_error_code",
    );
  }
  if (abort.surface_code !== null) {
    assertCompatibleSurfaceMutationRejectionCode(abort.surface_code);
  }
}

export function assertCompatibleSurfaceSessionResponse(
  value: unknown,
): asserts value is SurfaceSessionResponse {
  const response = responseWithStatus(
    value,
    SURFACE_SESSION_RESPONSE_STATUSES,
  );
  if (response.status === "started") {
    const session = record(response.session);
    assertTransferProtocolVersion(session.protocol_version);
    assertTransferProtocolVersion(record(session.payload).protocol_version);
  } else {
    assertCompatibleSurfaceTransferAbort(response.abort);
  }
}

export function assertCompatibleSurfaceTransferResponse(
  value: unknown,
): asserts value is SurfaceTransferResponse {
  const response = responseWithStatus(
    value,
    SURFACE_TRANSFER_RESPONSE_STATUSES,
  );
  if (response.status === "committed") {
    const completion = record(response.completion);
    assertTransferProtocolVersion(completion.protocol_version);
    assertCompatibleSurfaceTransferTarget(completion.target);
  } else {
    assertCompatibleSurfaceTransferAbort(response.abort);
  }
}

function responseWithStatus(
  value: unknown,
  statuses: readonly string[],
): Record<string, unknown> {
  const response = record(value);
  assertKnown(response.status, statuses, "unknown_response_status");
  return response;
}

function assertKnown(
  value: unknown,
  known: readonly string[],
  code: SurfaceTransferProtocolIncompatibilityCode,
): asserts value is string {
  if (typeof value !== "string" || !known.includes(value)) {
    throw new SurfaceTransferProtocolIncompatibilityError(code, value);
  }
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new SurfaceTransferProtocolIncompatibilityError(
      "unknown_response_status",
      value,
    );
  }
  return value as Record<string, unknown>;
}
