import { isOpaqueId } from "../../core/identifiers.ts";
import { checkWireFields, readWireRecord } from "../../core/record.ts";
import { CONFIG_OPERATIONS_PROTOCOL_VERSION } from "../generated/protocol.ts";

export type ConfigProtocolValidationCode =
  | "invalid_payload"
  | "unsupported_protocol"
  | "unknown_discriminant";

export class ConfigProtocolValidationError extends Error {
  readonly code: ConfigProtocolValidationCode;
  readonly path: string;

  constructor(code: ConfigProtocolValidationCode, path: string, message: string) {
    super(message);
    this.name = "ConfigProtocolValidationError";
    this.code = code;
    this.path = path;
  }
}

export function baseCommand(
  value: unknown,
  path: string,
  allowed?: ReadonlyArray<string>,
): Record<string, unknown> {
  const command = record(value, path, allowed);
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

export function record(
  value: unknown,
  path: string,
  allowed?: ReadonlyArray<string>,
): Record<string, unknown> {
  const result = readWireRecord(value, () => fail("invalid_payload", path, "expected object"));
  checkWireFields(result, allowed, (code, key) =>
    fail(
      "invalid_payload",
      `${path}.${key}`,
      code === "unknown_field" ? "unknown field" : "missing field",
    ),
  );
  return result;
}

export function array(value: unknown, path: string): Array<unknown> {
  if (!Array.isArray(value)) {
    fail("invalid_payload", path, "expected array");
  }
  return value;
}

export function string(value: unknown, path: string): asserts value is string {
  if (typeof value !== "string") {
    fail("invalid_payload", path, "expected string");
  }
}

export function nonempty(value: unknown, path: string): asserts value is string {
  string(value, path);
  if (value.length === 0) {
    fail("invalid_payload", path, "expected nonempty string");
  }
}

export function nullableString(value: unknown, path: string): void {
  if (value !== null) {
    string(value, path);
  }
}

export function digest(value: unknown, path: string): asserts value is string {
  string(value, path);
  if (!/^[0-9a-f]{64}$/.test(value)) {
    fail("invalid_payload", path, "expected lowercase SHA-256 digest");
  }
}

export function nullableDigest(value: unknown, path: string): void {
  if (value !== null) {
    digest(value, path);
  }
}

export function finiteNumber(value: unknown, path: string): asserts value is number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    fail("invalid_payload", path, "expected non-negative safe integer");
  }
}

export function boolean(value: unknown, path: string): asserts value is boolean {
  if (typeof value !== "boolean") {
    fail("invalid_payload", path, "expected boolean");
  }
}

export function discriminant(
  value: unknown,
  variants: ReadonlyArray<string>,
  path: string,
): asserts value is string {
  string(value, path);
  if (!variants.includes(value)) {
    fail("unknown_discriminant", path, `unknown value ${JSON.stringify(value)}`);
  }
}

export function fail(code: ConfigProtocolValidationCode, path: string, message: string): never {
  throw new ConfigProtocolValidationError(code, path, message);
}

function opaqueId(value: unknown, path: string): asserts value is string {
  if (!isOpaqueId(value)) {
    fail("invalid_payload", path, "expected bounded lowercase opaque id");
  }
}
