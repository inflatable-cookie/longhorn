import {
  CheckedSnapshotConnection,
  type ConnectionFailure,
  type ConnectionFailureReporter,
  type EventTransport,
} from "@inflatable-cookie/longhorn/core";

import type {
  SettingsLoadCommand,
  SettingsLoadOutcome,
  SettingsRegistryChangedEvent,
  SettingsRegistrySnapshot,
  SettingsRequestId,
  SettingsScopeChangedEvent,
  SettingsScopeId,
  SettingsScopeSnapshot,
} from "./generated/protocol.ts";
import {
  assertCompatibleSettingsRegistryChangedEvent,
  assertCompatibleSettingsRegistrySnapshot,
  assertCompatibleSettingsScopeChangedEvent,
  assertCompatibleSettingsScopeSnapshot,
} from "./validation.ts";
import {
  SETTINGS_REGISTRY_CHANGED_EVENT,
  SETTINGS_SCOPE_CHANGED_EVENT,
} from "./names.ts";

export interface SettingsSubscription<Snapshot> {
  readonly ready: Promise<Snapshot>;
  current(): Snapshot | undefined;
  failures(): readonly ConnectionFailure[];
  dispose(): Promise<void>;
}

export interface SettingsScopeConnectionOptions {
  readonly registry: SettingsRegistrySnapshot;
  readonly scopeId: SettingsScopeId;
  readonly nextRequestId: () => SettingsRequestId;
  readonly onSnapshot?: (snapshot: SettingsScopeSnapshot) => void;
  readonly onFailure?: ConnectionFailureReporter;
}

export function connectSettingsRegistry(
  transport: EventTransport,
  load: () => Promise<SettingsRegistrySnapshot>,
  listener?: (snapshot: SettingsRegistrySnapshot) => void,
  onFailure?: ConnectionFailureReporter,
): SettingsSubscription<SettingsRegistrySnapshot> {
  return requiredSubscription(
    new CheckedSnapshotConnection({
      listen: (receive) =>
        transport.listen(SETTINGS_REGISTRY_CHANGED_EVENT, receive),
      loadSnapshot: load,
      validateSnapshot: parseRegistry,
      handleEvent: registryEventAction,
      isNewer: isNewerRegistry,
      onSnapshot: listener,
      onFailure,
      disposedBeforeReadyError: () =>
        new SettingsConnectionDisposedError("registry"),
    }),
    "registry",
  );
}

export function connectSettingsScope(
  transport: EventTransport,
  load: (command: SettingsLoadCommand) => Promise<SettingsLoadOutcome>,
  options: SettingsScopeConnectionOptions,
): SettingsSubscription<SettingsScopeSnapshot> {
  let current: SettingsScopeSnapshot | undefined;
  const maximum = options.registry.limits.maximumOpaqueValueBytes;
  const connection = new CheckedSnapshotConnection<SettingsScopeSnapshot>({
    listen: (receive) =>
      transport.listen(SETTINGS_SCOPE_CHANGED_EVENT, receive),
    loadSnapshot: async () => {
      const outcome = await load({
        protocolVersion: 1,
        requestId: options.nextRequestId(),
        registryGeneration: options.registry.generation,
        scopeId: options.scopeId,
        knownAuthority: current?.authority ?? null,
      });
      if (outcome.status === "rejected") {
        throw new SettingsLoadRejectedError(outcome);
      }
      return outcome.snapshot;
    },
    validateSnapshot: (value) => {
      assertCompatibleSettingsScopeSnapshot(value, maximum);
      validateScopeIdentity(value, options);
      return value;
    },
    handleEvent: (value, snapshot) =>
      scopeEventAction(value, snapshot, options),
    isNewer: (candidate, snapshot) => {
      const accepted = isNewerScope(candidate, snapshot);
      if (accepted) {
        current = candidate;
      }
      return accepted;
    },
    onSnapshot: options.onSnapshot,
    onFailure: options.onFailure,
    disposedBeforeReadyError: () =>
      new SettingsConnectionDisposedError(options.scopeId),
  });
  return requiredSubscription(connection, options.scopeId);
}

export class SettingsConnectionDisposedError extends Error {
  constructor(target: string) {
    super(`settings ${target} connection was disposed during registration`);
    this.name = "SettingsConnectionDisposedError";
  }
}

export class SettingsLoadRejectedError extends Error {
  readonly outcome: SettingsLoadOutcome;

  constructor(outcome: SettingsLoadOutcome) {
    super("settings scope load was rejected");
    this.name = "SettingsLoadRejectedError";
    this.outcome = outcome;
  }
}

export class SettingsAuthorityConsistencyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SettingsAuthorityConsistencyError";
  }
}

function requiredSubscription<Snapshot>(
  connection: CheckedSnapshotConnection<Snapshot>,
  target: string,
): SettingsSubscription<Snapshot> {
  return {
    ready: connection.ready.then((snapshot) => {
      if (snapshot === undefined) {
        throw new SettingsConnectionDisposedError(target);
      }
      return snapshot;
    }),
    current: () => connection.current(),
    failures: () => connection.failures(),
    dispose: () => connection.dispose(),
  };
}

function parseRegistry(value: unknown): SettingsRegistrySnapshot {
  assertCompatibleSettingsRegistrySnapshot(value);
  return value;
}

function registryEventAction(
  value: unknown,
  current: SettingsRegistrySnapshot | undefined,
): { kind: "ignore" } | { kind: "refresh" } {
  assertCompatibleSettingsRegistryChangedEvent(value);
  const event = value as SettingsRegistryChangedEvent;
  return current !== undefined &&
    event.registryGeneration <= current.generation
    ? { kind: "ignore" }
    : { kind: "refresh" };
}

function scopeEventAction(
  value: unknown,
  current: SettingsScopeSnapshot | undefined,
  options: SettingsScopeConnectionOptions,
): { kind: "ignore" } | { kind: "refresh" } {
  assertCompatibleSettingsScopeChangedEvent(value);
  const event = value as SettingsScopeChangedEvent;
  if (event.scopeId !== options.scopeId) {
    return { kind: "ignore" };
  }
  const authority = current?.authority;
  return authority !== undefined &&
    (event.registryGeneration < authority.registryGeneration ||
      (event.registryGeneration === authority.registryGeneration &&
        event.scopeRevision <= authority.scopeRevision))
    ? { kind: "ignore" }
    : { kind: "refresh" };
}

function isNewerRegistry(
  candidate: SettingsRegistrySnapshot,
  current: SettingsRegistrySnapshot | undefined,
): boolean {
  if (current === undefined || candidate.generation > current.generation) {
    return true;
  }
  if (
    candidate.generation === current.generation &&
    candidate.digest !== current.digest
  ) {
    throw new SettingsAuthorityConsistencyError(
      "one settings registry generation produced multiple digests",
    );
  }
  return false;
}

function isNewerScope(
  candidate: SettingsScopeSnapshot,
  current: SettingsScopeSnapshot | undefined,
): boolean {
  if (current === undefined) {
    return true;
  }
  const next = candidate.authority;
  const previous = current.authority;
  if (next.registryGeneration !== previous.registryGeneration) {
    return next.registryGeneration > previous.registryGeneration;
  }
  if (next.scopeRevision !== previous.scopeRevision) {
    return next.scopeRevision > previous.scopeRevision;
  }
  if (next.authorityToken !== previous.authorityToken) {
    throw new SettingsAuthorityConsistencyError(
      "one settings scope revision produced multiple authority tokens",
    );
  }
  return false;
}

function validateScopeIdentity(
  snapshot: SettingsScopeSnapshot,
  options: SettingsScopeConnectionOptions,
): void {
  if (snapshot.scopeId !== options.scopeId) {
    throw new SettingsAuthorityConsistencyError(
      `loaded scope ${snapshot.scopeId}; expected ${options.scopeId}`,
    );
  }
  if (
    snapshot.authority.registryGeneration !== options.registry.generation
  ) {
    throw new SettingsAuthorityConsistencyError(
      "scope snapshot registry generation does not match the connection",
    );
  }
}
