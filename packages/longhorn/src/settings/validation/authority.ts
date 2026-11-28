import { SETTINGS_FIELDS } from "../generated/fields.ts";
import {
  SETTINGS_ACTIVATION_STATES,
  SETTINGS_EDITABILITIES,
  SETTINGS_EFFECTIVE_SOURCES,
  SETTINGS_HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
  SETTINGS_HARD_MAXIMUM_TEXT_BYTES,
  SETTINGS_POLICY_EFFECTS,
  SETTINGS_RECOVERY_CODES,
} from "../generated/protocol.ts";
import type { SettingsScopeSnapshot } from "../generated/protocol.ts";
import { incompatible } from "./error.ts";
import { activation, hasPolicyOverride, recovery } from "./projection.ts";
import {
  array,
  authority,
  identity,
  known,
  opaque,
  optionalOpaque,
  protocolVersion,
  record,
  text,
} from "./primitives.ts";

export function assertValidSettingsScopeSnapshot(
  value: unknown,
  maximumOpaqueValueBytes: number = SETTINGS_HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
): asserts value is SettingsScopeSnapshot {
  const snapshot = record(value, SETTINGS_FIELDS.SettingsScopeSnapshot);
  protocolVersion(snapshot.protocolVersion);
  identity(snapshot.scopeId);
  authority(snapshot.authority);
  const entries = new Set<string>();
  array(snapshot.values).forEach((value) => {
    const projection = record(value, SETTINGS_FIELDS.SettingsValueProjection);
    identity(projection.entryId);
    if (entries.has(projection.entryId)) {
      incompatible("invalid_shape", projection.entryId);
    }
    entries.add(projection.entryId);
    optionalOpaque(projection.configured, maximumOpaqueValueBytes);
    opaque(projection.effective, maximumOpaqueValueBytes);
    opaque(projection.compiledDefault, maximumOpaqueValueBytes);
    known(projection.effectiveSource, SETTINGS_EFFECTIVE_SOURCES);
    known(projection.editability, SETTINGS_EDITABILITIES);
    const hasOverride =
      projection.policy !== null &&
      hasPolicyOverride(projection.policy, maximumOpaqueValueBytes, SETTINGS_POLICY_EFFECTS);
    const managed = projection.effectiveSource === "managedPolicy";
    if (
      hasOverride !== managed ||
      (managed && projection.editability === "editable") ||
      (projection.effectiveSource === "userConfiguration" && projection.configured === null)
    ) {
      incompatible("invalid_shape", projection);
    }
    array(projection.sourceDiagnostics).forEach((value) => {
      const diagnostic = record(value, SETTINGS_FIELDS.SettingsSourceDiagnostic);
      text(diagnostic.code, SETTINGS_HARD_MAXIMUM_TEXT_BYTES);
      optionalOpaque(diagnostic.detail, maximumOpaqueValueBytes);
    });
  });
  if (snapshot.recovery !== null) {
    recovery(snapshot.recovery, maximumOpaqueValueBytes, SETTINGS_RECOVERY_CODES);
  }
  activation(snapshot.activationRequirements, SETTINGS_ACTIVATION_STATES);
}
