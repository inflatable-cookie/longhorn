import type { SettingsPageDefinition } from "@inflatable-cookie/longhorn/settings/protocol";

export class MissingSettingsRendererError extends Error {
  readonly page: SettingsPageDefinition;

  constructor(page: SettingsPageDefinition) {
    super(
      `settings renderer ${page.rendererId} is missing for page ${page.id}`,
    );
    this.name = "MissingSettingsRendererError";
    this.page = page;
  }
}

export class SettingsApplyUnitUnavailableError extends Error {
  constructor(applyUnitId: string) {
    super(`settings apply unit ${applyUnitId} is unavailable on this page`);
    this.name = "SettingsApplyUnitUnavailableError";
  }
}

export class SettingsDraftUnavailableError extends Error {
  constructor(applyUnitId: string) {
    super(`settings apply unit ${applyUnitId} has no staged draft`);
    this.name = "SettingsDraftUnavailableError";
  }
}

export class SettingsMutationPendingError extends Error {
  constructor() {
    super("settings mutation is still pending");
    this.name = "SettingsMutationPendingError";
  }
}

export class SettingsPageApplyAmbiguityError extends Error {
  constructor(count: number) {
    super(`page Apply cannot imply atomicity across ${count} apply units`);
    this.name = "SettingsPageApplyAmbiguityError";
  }
}

export class SettingsPageUnavailableError extends Error {
  constructor() {
    super("settings page is unavailable");
    this.name = "SettingsPageUnavailableError";
  }
}

export class SettingsRegistryUnavailableError extends Error {
  constructor() {
    super("settings registry is unavailable");
    this.name = "SettingsRegistryUnavailableError";
  }
}

export class SettingsRendererResolverUnavailableError extends Error {
  constructor() {
    super("settings renderer resolver is unavailable");
    this.name = "SettingsRendererResolverUnavailableError";
  }
}

export class SettingsResetUnsupportedError extends Error {
  constructor(applyUnitId: string) {
    super(`settings apply unit ${applyUnitId} does not support reset`);
    this.name = "SettingsResetUnsupportedError";
  }
}

export class SettingsScopeNotLoadedError extends Error {
  constructor(scopeId: string) {
    super(`settings scope ${scopeId} is not loaded for the current page`);
    this.name = "SettingsScopeNotLoadedError";
  }
}

export class SettingsScopeUnavailableError extends Error {
  constructor(scopeId: string) {
    super(`settings scope ${scopeId} has no current authority`);
    this.name = "SettingsScopeUnavailableError";
  }
}

