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
import { assertCompatibleSettingsScopeSnapshot } from "./authority.ts";
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
} from "./primitives.ts";

export function assertCompatibleSettingsLoadCommand(
  value: unknown,
): asserts value is SettingsLoadCommand {
  const command = record(value);
  protocolVersion(command.protocolVersion);
  identity(command.requestId);
  unsigned(command.registryGeneration, "invalid_revision");
  identity(command.scopeId);
  if (command.knownAuthority !== null) {
    authority(command.knownAuthority);
  }
}

export function assertCompatibleSettingsApplyCommand(
  value: unknown,
  maximumOpaqueValueBytes = HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
): asserts value is SettingsApplyCommand {
  const command = mutationCommand(value);
  opaque(command.intent, maximumOpaqueValueBytes);
}

export function assertCompatibleSettingsResetCommand(
  value: unknown,
): asserts value is SettingsResetCommand {
  const command = mutationCommand(value);
  array(command.entryIds).forEach(identity);
}

export function assertCompatibleSettingsLoadOutcome(
  value: unknown,
  maximumOpaqueValueBytes = HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
): asserts value is SettingsLoadOutcome {
  const outcome = record(value);
  known(outcome.status, SETTINGS_LOAD_OUTCOME_STATUSES);
  if (outcome.status === "loaded") {
    assertCompatibleSettingsScopeSnapshot(
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

export function assertCompatibleSettingsMutationResult(
  value: unknown,
  maximumOpaqueValueBytes = HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
): asserts value is SettingsMutationResult {
  const result = record(value);
  known(result.status, SETTINGS_MUTATION_RESULT_STATUSES);
  if (result.status === "applied") {
    assertCompatibleSettingsScopeSnapshot(
      result.snapshot,
      maximumOpaqueValueBytes,
    );
    receipt(result.receipt, maximumOpaqueValueBytes);
  } else if (result.status === "conflict") {
    const conflict = record(result.conflict);
    authority(conflict.expected);
    authority(conflict.actual);
    assertCompatibleSettingsScopeSnapshot(
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
      assertCompatibleSettingsScopeSnapshot(
        result.snapshot,
        maximumOpaqueValueBytes,
      );
    }
  }
}

export function assertCompatibleSettingsRegistryChangedEvent(
  value: unknown,
): asserts value is SettingsRegistryChangedEvent {
  const event = record(value);
  protocolVersion(event.protocolVersion);
  unsigned(event.registryGeneration, "invalid_revision");
}

export function assertCompatibleSettingsScopeChangedEvent(
  value: unknown,
): asserts value is SettingsScopeChangedEvent {
  const event = record(value);
  protocolVersion(event.protocolVersion);
  unsigned(event.registryGeneration, "invalid_revision");
  identity(event.scopeId);
  unsigned(event.scopeRevision, "invalid_revision");
}

function mutationCommand(value: unknown): Record<string, unknown> {
  const command = record(value);
  protocolVersion(command.protocolVersion);
  identity(command.requestId);
  identity(command.pageId);
  identity(command.applyUnitId);
  identity(command.scopeId);
  authority(command.authority);
  return command;
}

function receipt(value: unknown, maximum: number): void {
  const receipt = record(value);
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
