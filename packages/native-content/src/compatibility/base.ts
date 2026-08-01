import { NATIVE_CONTENT_PROTOCOL_VERSION } from "../generated/protocol.ts";

const FORBIDDEN_KEYS = new Set([
  "payload",
  "url",
  "navigation",
  "plugin",
  "midi",
  "camera",
  "renderer",
  "gpu",
  "raw_handle",
  "tauri_label",
]);

export class NativeContentProtocolCompatibilityError extends Error {
  constructor(readonly path: string, message: string) {
    super(`incompatible native-content protocol at ${path}: ${message}`);
    this.name = "NativeContentProtocolCompatibilityError";
  }
}

export function assertNativeContentProtocolVersion(
  value: unknown,
  path = "$.protocol_version",
): asserts value is typeof NATIVE_CONTENT_PROTOCOL_VERSION {
  if (value !== NATIVE_CONTENT_PROTOCOL_VERSION) {
    fail(path, `expected exact version ${NATIVE_CONTENT_PROTOCOL_VERSION}`);
  }
}

export function assertProductPayloadFree(value: unknown): void {
  visit(value, "$", new Set());
}

export function exactObject(
  value: unknown,
  path: string,
  keys: readonly string[],
): Record<string, unknown> {
  const object = record(value, path);
  exactKeys(object, path, keys);
  return object;
}

export function record(
  value: unknown,
  path: string,
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    fail(path, "expected object");
  }
  return value as Record<string, unknown>;
}

export function exactKeys(
  value: Record<string, unknown>,
  path: string,
  keys: readonly string[],
): void {
  const expected = new Set(keys);
  for (const key of Object.keys(value)) {
    if (!expected.has(key)) fail(`${path}.${key}`, "unknown field");
  }
  for (const key of keys) {
    if (!(key in value)) fail(`${path}.${key}`, "missing field");
  }
}

export function member<const Value extends string>(
  value: unknown,
  values: readonly Value[],
  path: string,
): asserts value is Value {
  if (typeof value !== "string" || !values.includes(value as Value)) {
    fail(path, `unknown value ${String(value)}`);
  }
}

export function text(value: unknown, path: string): asserts value is string {
  if (typeof value !== "string") fail(path, "expected string");
}

export function opaqueId(
  value: unknown,
  path: string,
): asserts value is string {
  text(value, path);
  if (
    value.length === 0 ||
    value.length > 128 ||
    !/^[a-z0-9._:-]+$/.test(value)
  ) {
    fail(path, "invalid bounded opaque id");
  }
}

export function boolean(
  value: unknown,
  path: string,
): asserts value is boolean {
  if (typeof value !== "boolean") fail(path, "expected boolean");
}

export function finite(
  value: unknown,
  path: string,
): asserts value is number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    fail(path, "expected finite number");
  }
}

export function natural(
  value: unknown,
  path: string,
): asserts value is number {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < 0
  ) {
    fail(path, "expected non-negative safe integer");
  }
}

export function positive(
  value: unknown,
  path: string,
): asserts value is number {
  natural(value, path);
  if (value === 0) fail(path, "expected positive safe integer");
}

export function array(value: unknown, path: string): readonly unknown[] {
  if (!Array.isArray(value)) fail(path, "expected array");
  return value;
}

export function nullable<T>(
  value: unknown,
  path: string,
  guard: (value: unknown, path: string) => asserts value is T,
): void {
  if (value !== null) guard(value, path);
}

export function fail(path: string, message: string): never {
  throw new NativeContentProtocolCompatibilityError(path, message);
}

function visit(value: unknown, path: string, seen: Set<object>): void {
  if (typeof value !== "object" || value === null) return;
  if (seen.has(value)) fail(path, "cyclic value");
  seen.add(value);
  if (Array.isArray(value)) {
    value.forEach((item, index) => visit(item, `${path}[${index}]`, seen));
  } else {
    for (const [key, item] of Object.entries(value)) {
      if (FORBIDDEN_KEYS.has(key)) fail(`${path}.${key}`, "product payload field");
      visit(item, `${path}.${key}`, seen);
    }
  }
  seen.delete(value);
}
