import {
  SETTINGS_VARIANT_FIELDS,
  SETTINGS_VARIANT_FIELDS_DISCRIMINANTS,
} from "../generated/variant-fields.ts";
import { SETTINGS_FIELDS } from "../generated/fields.ts";
import {
  SETTINGS_HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
  SETTINGS_PROTOCOL_VERSION,
  type SettingsOpaqueValue,
} from "../generated/protocol.ts";
import {
  incompatible,
  type SettingsProtocolValidationCode,
} from "./error.ts";

/**
 * Kept under its original name so the twenty call sites do not churn, but the
 * value is now the Rust `SettingsLimits::HARD_MAXIMUM_OPAQUE_VALUE_BYTES`
 * rather than a literal that happened to match it.
 */
export const HARD_MAXIMUM_OPAQUE_VALUE_BYTES: number =
  SETTINGS_HARD_MAXIMUM_OPAQUE_VALUE_BYTES;
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

export function opaque(
  value: unknown,
  maximum: number,
): asserts value is SettingsOpaqueValue {
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
  code: SettingsProtocolValidationCode,
): void {
  unsigned(value, code);
  if (value === 0) {
    incompatible(code, value);
  }
}

export function unsigned(
  value: unknown,
  code: SettingsProtocolValidationCode,
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
  code: SettingsProtocolValidationCode,
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

/**
 * Rejects a non-object, an unknown key, and a missing key.
 *
 * `allowed` comes from the generated field map, so the keys accepted are the
 * Rust struct's and nothing else — contract 010's Boundary Validation Target.
 * Called without a list, this keeps shape-only behaviour, which is correct
 * for a tagged union and a gap anywhere else.
 */
export function record(
  value: unknown,
  allowed?: readonly string[],
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    incompatible("invalid_shape", value);
  }
  const result = value as Record<string, unknown>;
  if (allowed === undefined) return result;

  const permitted = new Set(allowed);
  for (const key of Object.keys(result)) {
    if (!permitted.has(key)) {
      incompatible("unknown_field", { key, value });
    }
  }
  for (const key of allowed) {
    if (!(key in result)) {
      incompatible("missing_field", { key, value });
    }
  }
  return result;
}

/**
 * Allowed keys for one tagged-union variant, from the generated map.
 *
 * The discriminant's name comes from the map too: this domain tags
 * `SettingsDurabilityEvidence` on `kind` and its two results on `status`, and
 * a call site that has to name the right one is a per-site chance to name the
 * wrong one.
 *
 * A missing entry means the generator failed to read the union, not that a
 * caller sent something odd — every caller checks the discriminant with
 * `known()` above this call.
 */
export function variantKeys(
  type: string,
  value: Record<string, unknown>,
): readonly string[] {
  const discriminant = value[SETTINGS_VARIANT_FIELDS_DISCRIMINANTS[type] ?? "kind"];
  const keys = SETTINGS_VARIANT_FIELDS[type]?.[discriminant as string];
  if (keys === undefined) incompatible("unknown_field", { type, discriminant });
  return keys;
}
