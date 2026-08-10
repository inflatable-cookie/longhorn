export type SettingsProtocolIncompatibilityCode =
  | "invalid_shape"
  | "unsupported_protocol_version"
  | "invalid_identity"
  | "invalid_revision"
  | "invalid_registry"
  | "invalid_opaque_value"
  | "opaque_value_too_large"
  | "unknown_discriminant"
  | "unknown_field"
  | "missing_field";

export class SettingsProtocolIncompatibilityError extends Error {
  readonly code: SettingsProtocolIncompatibilityCode;
  readonly actual: unknown;

  constructor(
    code: SettingsProtocolIncompatibilityCode,
    actual: unknown,
  ) {
    super(`incompatible settings protocol: ${code}`);
    this.name = "SettingsProtocolIncompatibilityError";
    this.code = code;
    this.actual = actual;
  }
}

export function incompatible(
  code: SettingsProtocolIncompatibilityCode,
  actual: unknown,
): never {
  throw new SettingsProtocolIncompatibilityError(code, actual);
}
