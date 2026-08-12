import {
  SURFACE_TRANSFER_VARIANT_FIELDS,
  SURFACE_TRANSFER_VARIANT_FIELDS_DISCRIMINANTS,
} from "./generated/variant-fields.ts";
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
import { SURFACE_TRANSFER_FIELDS } from "./generated/fields.ts";
import {
  TRANSFER_ERROR_CODES,
  assertValidTransferTargetBinding,
  assertTransferProtocolVersion,
} from "@inflatable-cookie/longhorn/transfer";
import {
  assertValidSurfaceMutationRejectionCode,
} from "@inflatable-cookie/longhorn/surfaces";

export type SurfaceTransferProtocolValidationCode =
  | "unknown_target"
  | "unknown_abort_domain"
  | "unknown_surface_transfer_error_code"
  | "unknown_transfer_error_code"
  | "unknown_response_status"
  | "invalid_object"
  | "unknown_field"
  | "missing_field";

export class SurfaceTransferProtocolValidationError extends Error {
  readonly code: SurfaceTransferProtocolValidationCode;
  readonly received: unknown;

  constructor(
    code: SurfaceTransferProtocolValidationCode,
    received: unknown,
  ) {
    super(`incompatible Surface transfer protocol: ${code}`);
    this.name = "SurfaceTransferProtocolValidationError";
    this.code = code;
    this.received = received;
  }
}

export function assertValidSurfaceTransferTarget(
  value: unknown,
): asserts value is SurfaceTransferTarget {
  // The flat field map skips unions, because a union's allowed keys depend on
  // its discriminant and one list is wrong for every variant. That used to
  // mean no key check at all here; the generated per-variant map supplies one.
  const target = record(value);
  assertKnown(
    target.kind,
    SURFACE_TRANSFER_TARGET_KINDS,
    "unknown_target",
  );
  record(target, variantKeys("SurfaceTransferTarget", target));
  if (target.kind === "existing") {
    assertValidTransferTargetBinding(
      record(record(target.target).binding),
    );
  }
}

export function assertValidSurfaceTransferAbort(
  value: unknown,
): asserts value is SurfaceTransferAbort {
  const abort = record(value, SURFACE_TRANSFER_FIELDS.SurfaceTransferAbort);
  assertTransferProtocolVersion(abort.protocol_version);
  const source = record(abort.source);
  assertKnown(
    source.domain,
    SURFACE_TRANSFER_ABORT_DOMAINS,
    "unknown_abort_domain",
  );
  record(source, variantKeys("SurfaceTransferAbortSource", source));
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
    assertValidSurfaceMutationRejectionCode(abort.surface_code);
  }
}

export function assertValidSurfaceSessionResponse(
  value: unknown,
): asserts value is SurfaceSessionResponse {
  const response = responseWithStatus(
    value,
    SURFACE_SESSION_RESPONSE_STATUSES,
    "SurfaceSessionResponse",
  );
  if (response.status === "started") {
    const session = record(response.session);
    assertTransferProtocolVersion(session.protocol_version);
    assertTransferProtocolVersion(record(session.payload).protocol_version);
  } else {
    assertValidSurfaceTransferAbort(response.abort);
  }
}

export function assertValidSurfaceTransferResponse(
  value: unknown,
): asserts value is SurfaceTransferResponse {
  const response = responseWithStatus(
    value,
    SURFACE_TRANSFER_RESPONSE_STATUSES,
    "SurfaceTransferResponse",
  );
  if (response.status === "committed") {
    const completion = record(response.completion, SURFACE_TRANSFER_FIELDS.SurfaceTransferCompletion);
    assertTransferProtocolVersion(completion.protocol_version);
    assertValidSurfaceTransferTarget(completion.target);
  } else {
    assertValidSurfaceTransferAbort(response.abort);
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
 * discriminant's name read from the map too. This domain uses three: `kind`,
 * `status`, and `domain` on the abort source.
 *
 * A missing entry means the generator failed to read the union — every caller
 * runs `assertKnown` over the discriminant above this.
 */
function variantKeys(
  type: string,
  value: Record<string, unknown>,
): readonly string[] {
  const discriminant = value[SURFACE_TRANSFER_VARIANT_FIELDS_DISCRIMINANTS[type] ?? "kind"];
  const keys = SURFACE_TRANSFER_VARIANT_FIELDS[type]?.[discriminant as string];
  if (keys === undefined) {
    throw new SurfaceTransferProtocolValidationError("unknown_response_status", {
      type,
      discriminant,
    });
  }
  return keys;
}

function assertKnown(
  value: unknown,
  known: readonly string[],
  code: SurfaceTransferProtocolValidationCode,
): asserts value is string {
  if (typeof value !== "string" || !known.includes(value)) {
    throw new SurfaceTransferProtocolValidationError(code, value);
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
    throw new SurfaceTransferProtocolValidationError("invalid_object", value);
  }
  const result = value as Record<string, unknown>;
  if (allowed === undefined) return result;

  const permitted = new Set(allowed);
  for (const key of Object.keys(result)) {
    if (!permitted.has(key)) {
      throw new SurfaceTransferProtocolValidationError("unknown_field", { key, value });
    }
  }
  for (const key of allowed) {
    if (!(key in result)) {
      throw new SurfaceTransferProtocolValidationError("missing_field", { key, value });
    }
  }
  return result;
}
