import { SETTINGS_FIELDS } from "../generated/fields.ts";
import {
  SETTINGS_MUTATION_TIMINGS,
  type SettingsRegistrySnapshot,
} from "../generated/protocol.ts";
import { incompatible } from "./error.ts";
import {
  arraysOfIdentities,
  boolean,
  boundedArray,
  definition,
  identity,
  integer,
  known,
  positive,
  protocolVersion,
  record,
  text,
  unsigned,
  HARD_MAXIMUM_OPAQUE_VALUE_BYTES,
} from "./primitives.ts";

const DIGEST = /^[a-f0-9]{64}$/;

export function assertCompatibleSettingsRegistrySnapshot(
  value: unknown,
): asserts value is SettingsRegistrySnapshot {
  const registry = record(value, SETTINGS_FIELDS.SettingsRegistrySnapshot);
  protocolVersion(registry.protocolVersion);
  unsigned(registry.generation, "invalid_revision");
  if (typeof registry.digest !== "string" || !DIGEST.test(registry.digest)) {
    incompatible("invalid_registry", registry.digest);
  }
  const limits = settingsLimits(registry.limits);
  const modules = boundedArray(
    registry.modules,
    limits.maximumModules,
    "invalid_registry",
  );
  const sections = boundedArray(
    registry.sections,
    limits.maximumSections,
    "invalid_registry",
  );
  const pages = boundedArray(
    registry.pages,
    limits.maximumPages,
    "invalid_registry",
  );
  const renderers = boundedArray(
    registry.renderers,
    limits.maximumRenderers,
    "invalid_registry",
  );
  const scopes = boundedArray(
    registry.scopes,
    limits.maximumScopes,
    "invalid_registry",
  );
  const applyUnits = boundedArray(
    registry.applyUnits,
    limits.maximumApplyUnits,
    "invalid_registry",
  );
  const capabilities = boundedArray(
    registry.capabilities,
    limits.maximumCapabilities,
    "invalid_registry",
  );
  boundedArray(
    registry.composedCapabilities,
    limits.maximumCapabilities,
    "invalid_registry",
  ).forEach(identity);

  modules.forEach((value) => {
    const module = record(value, SETTINGS_FIELDS.SettingsModuleDefinition);
    identity(module.id);
    text(module.label, limits.maximumLabelBytes);
    integer(module.order);
  });
  sections.forEach((value) => {
    const section = record(value, SETTINGS_FIELDS.SettingsSectionDefinition);
    identity(section.id);
    identity(section.moduleId);
    text(section.label, limits.maximumLabelBytes);
    integer(section.order);
  });
  renderers.forEach(definition);
  scopes.forEach(definition);
  capabilities.forEach(definition);
  applyUnits.forEach((value) => {
    const unit = record(value, SETTINGS_FIELDS.SettingsApplyUnitDefinition);
    identity(unit.id);
    identity(unit.moduleId);
    identity(unit.scopeId);
    known(unit.timing, SETTINGS_MUTATION_TIMINGS);
    boolean(unit.resetSupported);
  });
  pages.forEach((value) => validatePage(value, limits));
}

function validatePage(
  value: unknown,
  limits: Record<string, number>,
): void {
  const page = record(value, SETTINGS_FIELDS.SettingsPageDefinition);
  identity(page.id);
  identity(page.moduleId);
  identity(page.sectionId);
  identity(page.rendererId);
  text(page.label, limits.maximumLabelBytes!);
  integer(page.order);
  boundedArray(
    page.keywords,
    limits.maximumKeywordsPerPage!,
    "invalid_registry",
  ).forEach((keyword) => text(keyword, limits.maximumKeywordBytes!));
  boundedArray(
    page.anchors,
    limits.maximumAnchorsPerPage!,
    "invalid_registry",
  ).forEach((value) => {
    const anchor = record(value, SETTINGS_FIELDS.SettingsAnchorDefinition);
    identity(anchor.id);
    if (anchor.label !== null) {
      text(anchor.label, limits.maximumLabelBytes!);
    }
    integer(anchor.order);
  });
  arraysOfIdentities(
    page.requiredCapabilities,
    page.readableScopeIds,
    page.writableApplyUnitIds,
  );
  const features = record(page.features, SETTINGS_FIELDS.SettingsPageFeatures);
  boolean(features.reset);
  boolean(features.import);
  boolean(features.backup);
  boolean(features.restore);
  boolean(features.confirmation);
}

function settingsLimits(value: unknown): Record<string, number> {
  const limits = record(value, SETTINGS_FIELDS.SettingsLimits);
  const names = [
    "maximumModules",
    "maximumSections",
    "maximumPages",
    "maximumRenderers",
    "maximumScopes",
    "maximumApplyUnits",
    "maximumCapabilities",
    "maximumAnchorsPerPage",
    "maximumKeywordsPerPage",
    "maximumLabelBytes",
    "maximumKeywordBytes",
    "maximumOpaqueValueBytes",
  ] as const;
  for (const name of names) {
    positive(limits[name], "invalid_registry");
  }
  if (
    (limits.maximumOpaqueValueBytes as number) >
    HARD_MAXIMUM_OPAQUE_VALUE_BYTES
  ) {
    incompatible("invalid_registry", limits.maximumOpaqueValueBytes);
  }
  return limits as Record<(typeof names)[number], number>;
}
