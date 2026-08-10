import {
  BRIDGE_MAXIMUM_DEDUPLICATION_ENTRIES,
  BRIDGE_MAXIMUM_FAILURE_MESSAGE_BYTES,
  BRIDGE_MAXIMUM_OPAQUE_ID_BYTES,
  BRIDGE_FAILURE_PHASES,
  BRIDGE_RETRY_CLASSES,
  type BridgeCancellationRequest,
  type BridgeCommandEnvelope,
  type BridgeCommandReply,
  type BridgeDeduplicationSupport,
  type BridgeFailure,
  type BridgeQueryEnvelope,
  type BridgeQueryReply,
  type BridgeRequestContext,
} from "../generated/protocol.ts";
import {
  type BridgeCodec,
  incompatible,
  integer,
  nullable,
  oneOf,
  opaqueId,
  record,
} from "./base.ts";

export function parseBridgeRequestContext(
  value: unknown,
): BridgeRequestContext {
  const source = record(value, ["requestId", "sessionId", "domainId"]);
  return {
    requestId: opaqueId(source.requestId),
    sessionId: opaqueId(source.sessionId),
    domainId: parseDomainId(source.domainId),
  };
}

export function parseBridgeQueryEnvelope<P>(
  value: unknown,
  payload: BridgeCodec<P>,
): BridgeQueryEnvelope<P> {
  const source = record(value, ["context", "payload"]);
  return {
    context: parseBridgeRequestContext(source.context),
    payload: payload.parse(source.payload),
  };
}

export function parseBridgeCommandEnvelope<P>(
  value: unknown,
  payload: BridgeCodec<P>,
): BridgeCommandEnvelope<P> {
  const source = record(value, [
    "context",
    "authorityEpoch",
    "expectedRevision",
    "idempotencyKey",
    "payload",
  ]);
  return {
    context: parseBridgeRequestContext(source.context),
    authorityEpoch: integer(source.authorityEpoch, 1),
    expectedRevision: nullable(source.expectedRevision, integer),
    idempotencyKey: nullable(source.idempotencyKey, opaqueId),
    payload: payload.parse(source.payload),
  };
}

export function parseBridgeCancellationRequest(
  value: unknown,
): BridgeCancellationRequest {
  const source = record(value, [
    "context",
    "targetRequestId",
    "jobId",
  ]);
  return {
    context: parseBridgeRequestContext(source.context),
    targetRequestId: opaqueId(source.targetRequestId),
    jobId: opaqueId(source.jobId),
  };
}

export function parseBridgeFailure<D>(
  value: unknown,
  details: BridgeCodec<D>,
): BridgeFailure<D> {
  const source = record(value, [
    "code",
    "message",
    "retryClass",
    "phase",
    "details",
  ]);
  if (
    typeof source.message !== "string" ||
    source.message.length === 0 ||
    new TextEncoder().encode(source.message).length >
      BRIDGE_MAXIMUM_FAILURE_MESSAGE_BYTES
  ) {
    incompatible("invalid_message", source.message);
  }
  return {
    code: opaqueId(source.code),
    message: source.message,
    retryClass: oneOf(
      source.retryClass,
      BRIDGE_RETRY_CLASSES,
      "unknown_retry_class",
    ),
    phase: oneOf(
      source.phase,
      BRIDGE_FAILURE_PHASES,
      "unknown_failure_phase",
    ),
    details: nullable(source.details, details.parse),
  };
}

export function parseBridgeDeduplicationSupport(
  value: unknown,
): BridgeDeduplicationSupport {
  if (value === "unsupported") {
    return value;
  }
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    Object.keys(value).length !== 1 ||
    !("finite" in value)
  ) {
    incompatible("unknown_deduplication_support", value);
  }
  const source = value as { finite: unknown };
  const capacity = integer(source.finite, 1);
  if (capacity > BRIDGE_MAXIMUM_DEDUPLICATION_ENTRIES) {
    incompatible("unknown_deduplication_support", value);
  }
  return { finite: capacity };
}

export function parseBridgeQueryReply<S, D>(
  value: unknown,
  success: BridgeCodec<S>,
  details: BridgeCodec<D>,
): BridgeQueryReply<S, D> {
  const source = record(value, ["requestId", "outcome"]);
  return {
    requestId: opaqueId(source.requestId),
    outcome: parseQueryOutcome(source.outcome, success, details),
  };
}

export function parseBridgeCommandReply<S, D>(
  value: unknown,
  success: BridgeCodec<S>,
  details: BridgeCodec<D>,
): BridgeCommandReply<S, D> {
  const source = record(value, [
    "requestId",
    "authoritativeRevision",
    "outcome",
  ]);
  return {
    requestId: opaqueId(source.requestId),
    authoritativeRevision: nullable(
      source.authoritativeRevision,
      integer,
    ),
    outcome: parseCommandOutcome(source.outcome, success, details),
  };
}

function parseQueryOutcome<S, D>(
  value: unknown,
  success: BridgeCodec<S>,
  details: BridgeCodec<D>,
): BridgeQueryReply<S, D>["outcome"] {
  const source = variant(value, ["success", "rejected"], "unknown_query_outcome");
  if ("success" in source) {
    return { success: success.parse(source.success) };
  }
  return { rejected: parseBridgeFailure(source.rejected, details) };
}

function parseCommandOutcome<S, D>(
  value: unknown,
  success: BridgeCodec<S>,
  details: BridgeCodec<D>,
): BridgeCommandReply<S, D>["outcome"] {
  const source = variant(
    value,
    ["applied", "rejected", "indeterminate"],
    "unknown_command_outcome",
  );
  if ("applied" in source) {
    return { applied: success.parse(source.applied) };
  }
  if ("rejected" in source) {
    return { rejected: parseBridgeFailure(source.rejected, details) };
  }
  return {
    indeterminate: parseBridgeFailure(source.indeterminate, details),
  };
}

function variant(
  value: unknown,
  keys: readonly string[],
  code: "unknown_query_outcome" | "unknown_command_outcome",
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    incompatible(code, value);
  }
  const source = value as Record<string, unknown>;
  const present = Object.keys(source);
  if (present.length !== 1 || !keys.includes(present[0]!)) {
    incompatible(code, value);
  }
  return source;
}

function parseDomainId(value: unknown): string {
  if (
    typeof value !== "string" ||
    new TextEncoder().encode(value).length > BRIDGE_MAXIMUM_OPAQUE_ID_BYTES ||
    !/^[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)*$/.test(value)
  ) {
    incompatible("invalid_domain_id", value);
  }
  return value;
}
