import fixtureJson from "../../../../fixtures/settings/protocol-v1.json";
import type {
  SettingsRegistrySnapshot,
  SettingsScopeSnapshot,
} from "../../src/settings/generated/protocol.ts";

export const fixture = fixtureJson;

export function registry(
  generation = fixture.registry.generation,
): SettingsRegistrySnapshot {
  const value = structuredClone(fixture.registry) as SettingsRegistrySnapshot;
  value.generation = generation;
  value.digest =
    generation === fixture.registry.generation
      ? fixture.registry.digest
      : generation.toString(16).padStart(64, "0");
  return value;
}

export function snapshot(
  revision = fixture.snapshots[0]!.authority.scopeRevision,
  generation = fixture.registry.generation,
): SettingsScopeSnapshot {
  const value = structuredClone(
    fixture.snapshots[0],
  ) as SettingsScopeSnapshot;
  value.authority.scopeRevision = revision;
  value.authority.registryGeneration = generation;
  value.authority.authorityToken = `authority:${generation}-${revision}`;
  return value;
}
