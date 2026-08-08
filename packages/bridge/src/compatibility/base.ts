import { BRIDGE_MAXIMUM_OPAQUE_ID_BYTES } from "../generated/protocol.ts";
export type BridgeProtocolIncompatibilityCode =
  | "unsupported_protocol_version"
  | "invalid_object"
  | "unknown_field"
  | "missing_field"
  | "invalid_id"
  | "invalid_domain_id"
  | "invalid_number"
  | "invalid_message"
  | "invalid_array"
  | "duplicate_value"
  | "unknown_host_form"
  | "unknown_connection_state"
  | "invalid_connection_status"
  | "unknown_authentication_posture"
  | "unknown_availability"
  | "unknown_read_authority"
  | "unknown_write_authority"
  | "unknown_execution_authority"
  | "invalid_authority_descriptor"
  | "unknown_failure_phase"
  | "unknown_retry_class"
  | "unknown_query_outcome"
  | "unknown_command_outcome"
  | "unknown_deduplication_support"
  | "unknown_job_outcome"
  | "unknown_cancellation_outcome"
  | "authority_without_capability"
  | "multiple_writers"
  | "unrequested_domain";

export class BridgeProtocolIncompatibilityError extends Error {
  readonly code: BridgeProtocolIncompatibilityCode;
  readonly received: unknown;

  constructor(
    code: BridgeProtocolIncompatibilityCode,
    received: unknown,
  ) {
    super(`incompatible bridge protocol: ${code}`);
    this.name = "BridgeProtocolIncompatibilityError";
    this.code = code;
    this.received = received;
  }
}

export interface BridgeCodec<T> {
  parse(value: unknown): T;
}

export function bridgeCodec<T>(
  parse: (value: unknown) => T,
): BridgeCodec<T> {
  return { parse };
}

export function record(
  value: unknown,
  required: readonly string[],
  optional: readonly string[] = [],
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    incompatible("invalid_object", value);
  }
  const result = value as Record<string, unknown>;
  const allowed = new Set([...required, ...optional]);
  for (const key of Object.keys(result)) {
    if (!allowed.has(key)) {
      incompatible("unknown_field", { key, value });
    }
  }
  for (const key of required) {
    if (!(key in result)) {
      incompatible("missing_field", { key, value });
    }
  }
  return result;
}

export function array(
  value: unknown,
  maximum: number,
): readonly unknown[] {
  if (!Array.isArray(value) || value.length > maximum) {
    incompatible("invalid_array", value);
  }
  return value;
}

export function unique<T>(
  values: readonly T[],
  key: (value: T) => string,
): void {
  const seen = new Set<string>();
  for (const value of values) {
    const identity = key(value);
    if (seen.has(identity)) {
      incompatible("duplicate_value", identity);
    }
    seen.add(identity);
  }
}

export function opaqueId(value: unknown): string {
  if (
    typeof value !== "string" ||
    new TextEncoder().encode(value).length > BRIDGE_MAXIMUM_OPAQUE_ID_BYTES ||
    !/^[a-z0-9._:-]+$/.test(value)
  ) {
    incompatible("invalid_id", value);
  }
  return value;
}

export function domainId(value: unknown): string {
  if (
    typeof value !== "string" ||
    new TextEncoder().encode(value).length > BRIDGE_MAXIMUM_OPAQUE_ID_BYTES ||
    !/^[a-z][a-z0-9_-]*(\.[a-z][a-z0-9_-]*)*$/.test(value)
  ) {
    incompatible("invalid_domain_id", value);
  }
  return value;
}

export function integer(
  value: unknown,
  minimum = 0,
): number {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < minimum
  ) {
    incompatible("invalid_number", value);
  }
  return value;
}

export function oneOf<const T extends string>(
  value: unknown,
  values: readonly T[],
  code: BridgeProtocolIncompatibilityCode,
): T {
  if (typeof value !== "string" || !values.includes(value as T)) {
    incompatible(code, value);
  }
  return value as T;
}

export function nullable<T>(
  value: unknown,
  parse: (value: unknown) => T,
): T | null {
  return value === null ? null : parse(value);
}

export function incompatible(
  code: BridgeProtocolIncompatibilityCode,
  received: unknown,
): never {
  throw new BridgeProtocolIncompatibilityError(code, received);
}
