import {
  SURFACE_MUTATION_COMMAND_KINDS,
  SURFACE_MUTATION_OUTCOME_KINDS,
  SURFACE_MUTATION_REJECTION_CODES,
  SURFACE_PROTOCOL_VERSION,
  type SurfaceChangedEvent,
  type SurfaceMutationCommand,
  type SurfaceMutationOutcome,
  type SurfaceMutationRejectionCode,
  type SurfaceMutationResponse,
  type SurfaceSnapshot,
} from "./generated/protocol.ts";
import { SURFACE_FIELDS } from "./generated/fields.ts";

export type SurfaceProtocolIncompatibilityCode =
  | "unsupported_protocol_version"
  | "unknown_command"
  | "unknown_outcome"
  | "unknown_rejection_code"
  | "unknown_response_status"
  | "invalid_object"
  | "unknown_field"
  | "missing_field";

export class SurfaceProtocolIncompatibilityError extends Error {
  readonly code: SurfaceProtocolIncompatibilityCode;
  readonly received: unknown;

  constructor(code: SurfaceProtocolIncompatibilityCode, received: unknown) {
    super(`incompatible Surface protocol: ${code}`);
    this.name = "SurfaceProtocolIncompatibilityError";
    this.code = code;
    this.received = received;
  }
}

export function assertSurfaceProtocolVersion(
  version: unknown,
): asserts version is typeof SURFACE_PROTOCOL_VERSION {
  if (version !== SURFACE_PROTOCOL_VERSION) {
    throw new SurfaceProtocolIncompatibilityError(
      "unsupported_protocol_version",
      version,
    );
  }
}

export function assertCompatibleSurfaceMutationCommand(
  value: unknown,
): asserts value is SurfaceMutationCommand {
  assertKnownKind(value, SURFACE_MUTATION_COMMAND_KINDS, "unknown_command");
}

export function assertCompatibleSurfaceMutationOutcome(
  value: unknown,
): asserts value is SurfaceMutationOutcome {
  assertKnownKind(value, SURFACE_MUTATION_OUTCOME_KINDS, "unknown_outcome");
}

export function assertCompatibleSurfaceMutationRejectionCode(
  value: unknown,
): asserts value is SurfaceMutationRejectionCode {
  if (
    typeof value !== "string" ||
    !SURFACE_MUTATION_REJECTION_CODES.includes(
      value as (typeof SURFACE_MUTATION_REJECTION_CODES)[number],
    )
  ) {
    throw new SurfaceProtocolIncompatibilityError(
      "unknown_rejection_code",
      value,
    );
  }
}

export function assertCompatibleSurfaceMutationResponse(
  value: unknown,
): asserts value is SurfaceMutationResponse {
  const response = record(value, SURFACE_FIELDS.SurfaceMutationResponse);
  switch (response.status) {
    case "committed": {
      const receipt = record(response.receipt, SURFACE_FIELDS.SurfaceMutationReceipt);
      assertCompatibleSurfaceMutationOutcome(record(receipt.outcome));
      return;
    }
    case "rejected": {
      const rejection = record(response.rejection, SURFACE_FIELDS.SurfaceMutationRejection);
      assertCompatibleSurfaceMutationRejectionCode(rejection.code);
      return;
    }
    default:
      throw new SurfaceProtocolIncompatibilityError(
        "unknown_response_status",
        response.status,
      );
  }
}

export function assertCompatibleSurfaceSnapshot(
  value: unknown,
): asserts value is SurfaceSnapshot {
  assertSurfaceProtocolVersion(
    record(value, SURFACE_FIELDS.SurfaceSnapshot).protocol_version,
  );
}

export function assertCompatibleSurfaceChangedEvent(
  value: unknown,
): asserts value is SurfaceChangedEvent {
  assertSurfaceProtocolVersion(
    record(value, SURFACE_FIELDS.SurfaceChangedEvent).protocol_version,
  );
}

function assertKnownKind(
  value: unknown,
  known: readonly string[],
  code: "unknown_command" | "unknown_outcome",
): asserts value is Record<"kind", string> {
  const candidate = record(value);
  if (
    typeof candidate.kind !== "string" ||
    !known.includes(candidate.kind)
  ) {
    throw new SurfaceProtocolIncompatibilityError(code, value);
  }
}

/**
 * Rejects a non-object, an unknown key, and a missing key.
 *
 * `allowed` comes from the generated field map, so the keys this accepts are
 * the Rust struct's and nothing else — contract 010's Boundary Validation
 * Target. Passing no list keeps the old shape-only behaviour for the tagged
 * unions, whose allowed keys depend on their discriminant and so are not one
 * flat set.
 *
 * The non-object case used to throw `unknown_response_status`, because the
 * incompatibility union had no code for it. It reported the wrong thing for
 * every caller but one.
 */
function record(
  value: unknown,
  allowed?: readonly string[],
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new SurfaceProtocolIncompatibilityError("invalid_object", value);
  }
  const result = value as Record<string, unknown>;
  if (allowed === undefined) return result;

  const permitted = new Set(allowed);
  for (const key of Object.keys(result)) {
    if (!permitted.has(key)) {
      throw new SurfaceProtocolIncompatibilityError("unknown_field", {
        key,
        value,
      });
    }
  }
  for (const key of allowed) {
    if (!(key in result)) {
      throw new SurfaceProtocolIncompatibilityError("missing_field", {
        key,
        value,
      });
    }
  }
  return result;
}
