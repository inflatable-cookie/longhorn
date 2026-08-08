import {
  SETTINGS_PROTOCOL_VERSION,
  type SettingsOpaqueValue,
} from "../generated/protocol.ts";
import {
  incompatible,
  type SettingsProtocolIncompatibilityCode,
} from "./error.ts";

export const HARD_MAXIMUM_OPAQUE_VALUE_BYTES = 1_048_576;
const OPAQUE_ID = /^[a-z0-9][a-z0-9._:/-]{0,254}$/;

export function authority(value: unknown): void {
  const valueRecord = record(value);
  unsigned(valueRecord.registryGeneration, "invalid_revision");
  unsigned(valueRecord.scopeRevision, "invalid_revision");
  identity(valueRecord.authorityToken);
}

export function optionalOpaque(value: unknown, maximum: number): void {
  if (value !== null) {
    opaque(value, maximum);
  }
}

export function opaque(
  value: unknown,
  maximum: number,
): asserts value is SettingsOpaqueValue {
  const envelope = record(value);
  positive(envelope.codecVersion, "invalid_opaque_value");
  let encoded: string | undefined;
  try {
    encoded = JSON.stringify(envelope);
  } catch {
    incompatible("invalid_opaque_value", value);
  }
  if (encoded === undefined) {
    incompatible("invalid_opaque_value", value);
  }
  const bytes = new TextEncoder().encode(encoded).byteLength;
  if (bytes > maximum || bytes > HARD_MAXIMUM_OPAQUE_VALUE_BYTES) {
    incompatible("opaque_value_too_large", bytes);
  }
}

export function definition(value: unknown): void {
  const valueRecord = record(value);
  identity(valueRecord.id);
  identity(valueRecord.moduleId);
}

export function arraysOfIdentities(...values: unknown[]): void {
  values.forEach((value) => array(value).forEach(identity));
}

export function identity(value: unknown): void {
  if (typeof value !== "string" || !OPAQUE_ID.test(value)) {
    incompatible("invalid_identity", value);
  }
}

export function protocolVersion(value: unknown): void {
  if (value !== SETTINGS_PROTOCOL_VERSION) {
    incompatible("unsupported_protocol_version", value);
  }
}

export function known(value: unknown, choices: readonly string[]): void {
  if (typeof value !== "string" || !choices.includes(value)) {
    incompatible("unknown_discriminant", value);
  }
}

export function text(value: unknown, maximumBytes: number): void {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    new TextEncoder().encode(value).byteLength > maximumBytes
  ) {
    incompatible("invalid_registry", value);
  }
}

export function boolean(value: unknown): void {
  if (typeof value !== "boolean") {
    incompatible("invalid_shape", value);
  }
}

export function integer(value: unknown): void {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    incompatible("invalid_shape", value);
  }
}

export function positive(
  value: unknown,
  code: SettingsProtocolIncompatibilityCode,
): void {
  unsigned(value, code);
  if (value === 0) {
    incompatible(code, value);
  }
}

export function unsigned(
  value: unknown,
  code: SettingsProtocolIncompatibilityCode,
): void {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < 0
  ) {
    incompatible(code, value);
  }
}

export function boundedArray(
  value: unknown,
  maximum: number,
  code: SettingsProtocolIncompatibilityCode,
): unknown[] {
  const values = array(value);
  if (values.length > maximum) {
    incompatible(code, values.length);
  }
  return values;
}

export function array(value: unknown): unknown[] {
  if (!Array.isArray(value)) {
    incompatible("invalid_shape", value);
  }
  return value;
}

export function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    incompatible("invalid_shape", value);
  }
  return value as Record<string, unknown>;
}
