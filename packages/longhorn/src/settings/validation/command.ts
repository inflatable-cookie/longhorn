import { SETTINGS_FIELDS } from "../generated/fields.ts";
import {
  SETTINGS_ACTIVATION_STATES,
  SETTINGS_DURABILITY_KINDS,
  SETTINGS_LOAD_OUTCOME_STATUSES,
  SETTINGS_MUTATION_OUTCOMES,
  SETTINGS_MUTATION_RESULT_STATUSES,
  SETTINGS_REJECTION_CODES,
  type SettingsApplyCommand,
  type SettingsLoadCommand,
  type SettingsLoadOutcome,
  type SettingsMutationResult,
  type SettingsRegistryChangedEvent,
  type SettingsResetCommand,
  type SettingsScopeChangedEvent,
} from "../generated/protocol.ts";
import { assertValidSettingsScopeSnapshot } from "./authority.ts";
import { activation, rejection } from "./projection.ts";
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
  unsigned,
  variantKeys,
} from "./primitives.ts";

export function assertValidSettingsLoadCommand(
  value: unknown,
): asserts value is SettingsLoadCommand {
  const command = record(value, SETTINGS_FIELDS.SettingsLoadCommand);
  protocolVersion(command.protocolVersion);
  identity(command.requestId);
  unsigned(command.registryGeneration, "invalid_revision");
  identity(command.scopeId);
  if (command.knownAuthority !== null) {
    authority(command.knownAuthority);
  }
}

export function assertValidSettingsApplyCommand(
  value: unknown,
  maximumOpaqueValueBytes = HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
): asserts value is SettingsApplyCommand {
  const command = mutationCommand(value, SETTINGS_FIELDS.SettingsApplyCommand);
  opaque(command.intent, maximumOpaqueValueBytes);
}

export function assertValidSettingsResetCommand(
  value: unknown,
): asserts value is SettingsResetCommand {
  const command = mutationCommand(value, SETTINGS_FIELDS.SettingsResetCommand);
  array(command.entryIds).forEach(identity);
}

export function assertValidSettingsLoadOutcome(
  value: unknown,
  maximumOpaqueValueBytes = HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
): asserts value is SettingsLoadOutcome {
  const outcome = record(value);
  known(outcome.status, SETTINGS_LOAD_OUTCOME_STATUSES);
  record(outcome, variantKeys("SettingsLoadOutcome", outcome));
  if (outcome.status === "loaded") {
    assertValidSettingsScopeSnapshot(
      outcome.snapshot,
      maximumOpaqueValueBytes,
    );
  } else {
    rejection(
      outcome.rejection,
      maximumOpaqueValueBytes,
      SETTINGS_REJECTION_CODES,
    );
  }
}

export function assertValidSettingsMutationResult(
  value: unknown,
  maximumOpaqueValueBytes = HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
): asserts value is SettingsMutationResult {
  const result = record(value);
  known(result.status, SETTINGS_MUTATION_RESULT_STATUSES);
  record(result, variantKeys("SettingsMutationResult", result));
  if (result.status === "applied") {
    assertValidSettingsScopeSnapshot(
      result.snapshot,
      maximumOpaqueValueBytes,
    );
    receipt(result.receipt, maximumOpaqueValueBytes);
  } else if (result.status === "conflict") {
    const conflict = record(result.conflict, SETTINGS_FIELDS.SettingsConflict);
    authority(conflict.expected);
    authority(conflict.actual);
    assertValidSettingsScopeSnapshot(
      result.snapshot,
      maximumOpaqueValueBytes,
    );
  } else {
    rejection(
      result.rejection,
      maximumOpaqueValueBytes,
      SETTINGS_REJECTION_CODES,
    );
    if (result.snapshot !== null) {
      assertValidSettingsScopeSnapshot(
        result.snapshot,
        maximumOpaqueValueBytes,
      );
    }
  }
}

export function assertValidSettingsRegistryChangedEvent(
  value: unknown,
): asserts value is SettingsRegistryChangedEvent {
  const event = record(value, SETTINGS_FIELDS.SettingsRegistryChangedEvent);
  protocolVersion(event.protocolVersion);
  unsigned(event.registryGeneration, "invalid_revision");
}

export function assertValidSettingsScopeChangedEvent(
  value: unknown,
): asserts value is SettingsScopeChangedEvent {
  const event = record(value, SETTINGS_FIELDS.SettingsScopeChangedEvent);
  protocolVersion(event.protocolVersion);
  unsigned(event.registryGeneration, "invalid_revision");
  identity(event.scopeId);
  unsigned(event.scopeRevision, "invalid_revision");
}

function mutationCommand(
  value: unknown,
  allowed?: readonly string[],
): Record<string, unknown> {
  const command = record(value, allowed);
  protocolVersion(command.protocolVersion);
  identity(command.requestId);
  identity(command.pageId);
  identity(command.applyUnitId);
  identity(command.scopeId);
  authority(command.authority);
  return command;
}

function receipt(value: unknown, maximum: number): void {
  const receipt = record(value, SETTINGS_FIELDS.SettingsMutationReceipt);
  identity(receipt.requestId);
  identity(receipt.pageId);
  identity(receipt.applyUnitId);
  identity(receipt.scopeId);
  authority(receipt.previousAuthority);
  authority(receipt.committedAuthority);
  known(receipt.outcome, SETTINGS_MUTATION_OUTCOMES);
  const durability = record(receipt.durability);
  known(durability.kind, SETTINGS_DURABILITY_KINDS);
  if (durability.kind === "confirmed") {
    optionalOpaque(durability.evidence, maximum);
  }
  activation(receipt.activationRequirements, SETTINGS_ACTIVATION_STATES);
}
