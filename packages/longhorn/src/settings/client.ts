import type {
  ConnectionFailureReporter,
  EventTransport,
} from "@inflatable-cookie/longhorn/core";

import type {
  SettingsApplyCommand,
  SettingsLoadCommand,
  SettingsLoadOutcome,
  SettingsMutationResult,
  SettingsRegistrySnapshot,
  SettingsResetCommand,
  SettingsScopeSnapshot,
} from "./generated/protocol.ts";
import {
  assertValidSettingsApplyCommand,
  assertValidSettingsLoadCommand,
  assertValidSettingsLoadOutcome,
  assertValidSettingsMutationResult,
  assertValidSettingsRegistrySnapshot,
  assertValidSettingsResetCommand,
} from "./validation.ts";
import {
  connectSettingsRegistry,
  connectSettingsScope,
  SettingsAuthorityConsistencyError,
  type SettingsScopeConnectionOptions,
  type SettingsSubscription,
} from "./connection.ts";
import {
  SETTINGS_APPLY_COMMAND,
  SETTINGS_LOAD_COMMAND,
  SETTINGS_REGISTRY_COMMAND,
  SETTINGS_RESET_COMMAND,
} from "./names.ts";

export class SettingsClient {
  readonly #transport: EventTransport;

  constructor(transport: EventTransport) {
    this.#transport = transport;
  }

  async registry(): Promise<SettingsRegistrySnapshot> {
    const value = await this.#transport.invoke(
      SETTINGS_REGISTRY_COMMAND,
      {},
    );
    assertValidSettingsRegistrySnapshot(value);
    return value;
  }

  async load(
    registry: SettingsRegistrySnapshot,
    command: SettingsLoadCommand,
  ): Promise<SettingsLoadOutcome> {
    assertValidSettingsLoadCommand(command);
    assertRegistryGeneration(registry, command.registryGeneration);
    const value = await this.#transport.invoke(SETTINGS_LOAD_COMMAND, {
      command,
    });
    assertValidSettingsLoadOutcome(
      value,
      registry.limits.maximumOpaqueValueBytes,
    );
    return value;
  }

  async apply(
    registry: SettingsRegistrySnapshot,
    command: SettingsApplyCommand,
  ): Promise<SettingsMutationResult> {
    assertValidSettingsApplyCommand(
      command,
      registry.limits.maximumOpaqueValueBytes,
    );
    assertRegistryGeneration(
      registry,
      command.authority.registryGeneration,
    );
    const value = await this.#transport.invoke(SETTINGS_APPLY_COMMAND, {
      command,
    });
    assertValidSettingsMutationResult(
      value,
      registry.limits.maximumOpaqueValueBytes,
    );
    return value;
  }

  async reset(
    registry: SettingsRegistrySnapshot,
    command: SettingsResetCommand,
  ): Promise<SettingsMutationResult> {
    assertValidSettingsResetCommand(command);
    assertRegistryGeneration(
      registry,
      command.authority.registryGeneration,
    );
    const value = await this.#transport.invoke(SETTINGS_RESET_COMMAND, {
      command,
    });
    assertValidSettingsMutationResult(
      value,
      registry.limits.maximumOpaqueValueBytes,
    );
    return value;
  }

  connectRegistry(
    listener?: (snapshot: SettingsRegistrySnapshot) => void,
    onFailure?: ConnectionFailureReporter,
  ): SettingsSubscription<SettingsRegistrySnapshot> {
    return connectSettingsRegistry(
      this.#transport,
      () => this.registry(),
      listener,
      onFailure,
    );
  }

  connectScope(
    options: SettingsScopeConnectionOptions,
  ): SettingsSubscription<SettingsScopeSnapshot> {
    return connectSettingsScope(
      this.#transport,
      (command) => this.load(options.registry, command),
      options,
    );
  }
}

function assertRegistryGeneration(
  registry: SettingsRegistrySnapshot,
  generation: number,
): void {
  if (generation !== registry.generation) {
    throw new SettingsAuthorityConsistencyError(
      "settings command registry generation does not match its registry",
    );
  }
}
