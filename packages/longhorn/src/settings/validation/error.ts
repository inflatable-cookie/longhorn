export type SettingsProtocolValidationCode =
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

export class SettingsProtocolValidationError extends Error {
  readonly code: SettingsProtocolValidationCode;
  readonly actual: unknown;

  constructor(
    code: SettingsProtocolValidationCode,
    actual: unknown,
  ) {
    super(`incompatible settings protocol: ${code}`);
    this.name = "SettingsProtocolValidationError";
    this.code = code;
    this.actual = actual;
  }
}

export function incompatible(
  code: SettingsProtocolValidationCode,
  actual: unknown,
): never {
  throw new SettingsProtocolValidationError(code, actual);
}
