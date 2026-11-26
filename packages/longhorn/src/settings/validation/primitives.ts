import { checkWireFields, readWireRecord, wireVariantFields } from "../../core/record.ts";
import {
  SETTINGS_VARIANT_FIELDS,
  SETTINGS_VARIANT_FIELDS_DISCRIMINANTS,
} from "../generated/variant-fields.ts";
import { SETTINGS_FIELDS } from "../generated/fields.ts";
import {
  SETTINGS_HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
  SETTINGS_PROTOCOL_VERSION,
} from "../generated/protocol.ts";
import type { SettingsOpaqueValue } from "../generated/protocol.ts";
import { incompatible } from "./error.ts";
import type { SettingsProtocolValidationCode } from "./error.ts";

/**
 * Kept under its original name so the twenty call sites do not churn, but the
 * value is now the Rust `SettingsLimits::HARD_MAXIMUM_OPAQUE_VALUE_BYTES`
 * rather than a literal that happened to match it.
 */
export const HARD_MAXIMUM_OPAQUE_VALUE_BYTES: number = SETTINGS_HARD_MAXIMUM_OPAQUE_VALUE_BYTES;
const OPAQUE_ID = /^[a-z0-9][a-z0-9._:/-]{0,254}$/;

export function authority(value: unknown): void {
  const valueRecord = record(value, SETTINGS_FIELDS.SettingsAuthorityExpectation);
  unsigned(valueRecord.registryGeneration, "invalid_revision");
  unsigned(valueRecord.scopeRevision, "invalid_revision");
  identity(valueRecord.authorityToken);
}

export function optionalOpaque(value: unknown, maximum: number): void {
  if (value !== null) {
    opaque(value, maximum);
  }
}

export function opaque(value: unknown, maximum: number): asserts value is SettingsOpaqueValue {
  const envelope = record(value, SETTINGS_FIELDS.SettingsOpaqueValue);
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

export function arraysOfIdentities(...values: Array<unknown>): void {
  values.forEach((value) => array(value).forEach(identity));
}

export function identity(value: unknown): asserts value is string {
  if (typeof value !== "string" || !OPAQUE_ID.test(value)) {
    incompatible("invalid_identity", value);
  }
}

export function protocolVersion(value: unknown): void {
  if (value !== SETTINGS_PROTOCOL_VERSION) {
    incompatible("unsupported_protocol_version", value);
  }
}

export function known(value: unknown, choices: ReadonlyArray<string>): void {
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

export function positive(value: unknown, code: SettingsProtocolValidationCode): void {
  unsigned(value, code);
  if (value === 0) {
    incompatible(code, value);
  }
}

export function unsigned(value: unknown, code: SettingsProtocolValidationCode): void {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    incompatible(code, value);
  }
}

export function boundedArray(
  value: unknown,
  maximum: number,
  code: SettingsProtocolValidationCode,
): Array<unknown> {
  const values = array(value);
  if (values.length > maximum) {
    incompatible(code, values.length);
  }
  return values;
}

export function array(value: unknown): Array<unknown> {
  if (!Array.isArray(value)) {
    incompatible("invalid_shape", value);
  }
  return value;
}

export function record(value: unknown, allowed?: ReadonlyArray<string>): Record<string, unknown> {
  const result = readWireRecord(value, () => incompatible("invalid_shape", value));
  checkWireFields(result, allowed, (code, key) => incompatible(code, { key, value }));
  return result;
}

export function variantKeys(type: string, value: Record<string, unknown>): ReadonlyArray<string> {
  return wireVariantFields(
    SETTINGS_VARIANT_FIELDS,
    SETTINGS_VARIANT_FIELDS_DISCRIMINANTS,
    type,
    value,
    (discriminant) => incompatible("unknown_field", { type, discriminant }),
  );
}
