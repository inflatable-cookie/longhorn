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

export type SurfaceProtocolIncompatibilityCode =
  | "unsupported_protocol_version"
  | "unknown_command"
  | "unknown_outcome"
  | "unknown_rejection_code"
  | "unknown_response_status";

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
  const response = record(value);
  switch (response.status) {
    case "committed": {
      const receipt = record(response.receipt);
      assertCompatibleSurfaceMutationOutcome(record(receipt.outcome));
      return;
    }
    case "rejected": {
      const rejection = record(response.rejection);
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
  assertSurfaceProtocolVersion(record(value).protocol_version);
}

export function assertCompatibleSurfaceChangedEvent(
  value: unknown,
): asserts value is SurfaceChangedEvent {
  assertSurfaceProtocolVersion(record(value).protocol_version);
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

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new SurfaceProtocolIncompatibilityError(
      "unknown_response_status",
      value,
    );
  }
  return value as Record<string, unknown>;
}
