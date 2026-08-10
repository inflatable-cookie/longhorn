import { SETTINGS_FIELDS } from "../generated/fields.ts";
import {
  SETTINGS_ACTIVATION_STATES,
  SETTINGS_EDITABILITIES,
  SETTINGS_EFFECTIVE_SOURCES,
  SETTINGS_POLICY_EFFECTS,
  SETTINGS_RECOVERY_CODES,
  type SettingsScopeSnapshot,
} from "../generated/protocol.ts";
import {
  activation,
  policy,
  recovery,
} from "./projection.ts";
import {
  array,
  authority,
  HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
  identity,
  known,
  opaque,
  optionalOpaque,
  protocolVersion,
  record,
  text,
} from "./primitives.ts";

export function assertCompatibleSettingsScopeSnapshot(
  value: unknown,
  maximumOpaqueValueBytes = HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
): asserts value is SettingsScopeSnapshot {
  const snapshot = record(value, SETTINGS_FIELDS.SettingsScopeSnapshot);
  protocolVersion(snapshot.protocolVersion);
  identity(snapshot.scopeId);
  authority(snapshot.authority);
  array(snapshot.values).forEach((value) => {
    const projection = record(value);
    identity(projection.entryId);
    optionalOpaque(projection.configured, maximumOpaqueValueBytes);
    opaque(projection.effective, maximumOpaqueValueBytes);
    opaque(projection.compiledDefault, maximumOpaqueValueBytes);
    known(projection.effectiveSource, SETTINGS_EFFECTIVE_SOURCES);
    known(projection.editability, SETTINGS_EDITABILITIES);
    if (projection.policy !== null) {
      policy(
        projection.policy,
        maximumOpaqueValueBytes,
        SETTINGS_POLICY_EFFECTS,
      );
    }
    array(projection.sourceDiagnostics).forEach((value) => {
      const diagnostic = record(value);
      text(diagnostic.code, 16_384);
      optionalOpaque(diagnostic.detail, maximumOpaqueValueBytes);
    });
  });
  if (snapshot.recovery !== null) {
    recovery(
      snapshot.recovery,
      maximumOpaqueValueBytes,
      SETTINGS_RECOVERY_CODES,
    );
  }
  activation(snapshot.activationRequirements, SETTINGS_ACTIVATION_STATES);
}
