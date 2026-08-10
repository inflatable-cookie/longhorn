import { CONFIG_MAXIMUM_OPAQUE_ID_BYTES } from "../generated/base.ts";
import { CONFIG_OPERATIONS_PROTOCOL_VERSION } from "../generated/protocol.ts";

export type ConfigProtocolIncompatibilityCode =
  | "invalid_payload"
  | "unsupported_protocol"
  | "unknown_discriminant";

export class ConfigProtocolIncompatibilityError extends Error {
  readonly code: ConfigProtocolIncompatibilityCode;
  readonly path: string;

  constructor(
    code: ConfigProtocolIncompatibilityCode,
    path: string,
    message: string,
  ) {
    super(message);
    this.name = "ConfigProtocolIncompatibilityError";
    this.code = code;
    this.path = path;
  }
}

export function baseCommand(
  value: unknown,
  path: string,
): Record<string, unknown> {
  const command = record(value, path);
  protocol(command.protocolVersion, `${path}.protocolVersion`);
  opaqueId(command.requestId, `${path}.requestId`);
  return command;
}

export function protocol(value: unknown, path: string): void {
  if (value !== CONFIG_OPERATIONS_PROTOCOL_VERSION) {
    fail(
      "unsupported_protocol",
      path,
      `expected config protocol ${CONFIG_OPERATIONS_PROTOCOL_VERSION}`,
    );
  }
}

/**
 * Rejects a non-object, an unknown key, and a missing key.
 *
 * `allowed` comes from the generated field map, so the keys accepted are the
 * Rust struct's and nothing else — contract 010's Boundary Validation Target.
 * `path` is kept ahead of it because every existing caller passes one and it
 * is what makes a rejection locatable in a nested document.
 *
 * Called without a list, this keeps shape-only behaviour. That is correct for
 * a tagged union, whose allowed keys depend on its discriminant, and it is a
 * gap anywhere else — the remaining untyped call sites are the work left.
 */
export function record(
  value: unknown,
  path: string,
  allowed?: readonly string[],
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    fail("invalid_payload", path, "expected object");
  }
  const result = value as Record<string, unknown>;
  if (allowed === undefined) return result;

  const permitted = new Set(allowed);
  for (const key of Object.keys(result)) {
    if (!permitted.has(key)) {
      fail("invalid_payload", `${path}.${key}`, "unknown field");
    }
  }
  for (const key of allowed) {
    if (!(key in result)) {
      fail("invalid_payload", `${path}.${key}`, "missing field");
    }
  }
  return result;
}

export function array(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) fail("invalid_payload", path, "expected array");
  return value;
}

export function string(
  value: unknown,
  path: string,
): asserts value is string {
  if (typeof value !== "string") {
    fail("invalid_payload", path, "expected string");
  }
}

export function nonempty(
  value: unknown,
  path: string,
): asserts value is string {
  string(value, path);
  if (value.length === 0) {
    fail("invalid_payload", path, "expected nonempty string");
  }
}

export function nullableString(value: unknown, path: string): void {
  if (value !== null) string(value, path);
}

export function digest(
  value: unknown,
  path: string,
): asserts value is string {
  string(value, path);
  if (!/^[0-9a-f]{64}$/.test(value)) {
    fail("invalid_payload", path, "expected lowercase SHA-256 digest");
  }
}

export function nullableDigest(value: unknown, path: string): void {
  if (value !== null) digest(value, path);
}

export function opaqueId(
  value: unknown,
  path: string,
): asserts value is string {
  string(value, path);
  if (
    value.length === 0 ||
    value.length > CONFIG_MAXIMUM_OPAQUE_ID_BYTES ||
    !/^[a-z0-9._:-]+$/.test(value)
  ) {
    fail("invalid_payload", path, "expected bounded lowercase opaque id");
  }
}

export function finiteNumber(
  value: unknown,
  path: string,
): asserts value is number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    fail("invalid_payload", path, "expected non-negative safe integer");
  }
}

export function boolean(
  value: unknown,
  path: string,
): asserts value is boolean {
  if (typeof value !== "boolean") {
    fail("invalid_payload", path, "expected boolean");
  }
}

export function discriminant(
  value: unknown,
  variants: readonly string[],
  path: string,
): asserts value is string {
  string(value, path);
  if (!variants.includes(value)) {
    fail("unknown_discriminant", path, `unknown value ${JSON.stringify(value)}`);
  }
}

export function fail(
  code: ConfigProtocolIncompatibilityCode,
  path: string,
  message: string,
): never {
  throw new ConfigProtocolIncompatibilityError(code, path, message);
}
