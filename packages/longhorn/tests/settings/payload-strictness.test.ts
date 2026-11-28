import { describe, expect, test } from "bun:test";

import {
  assertValidSettingsApplyCommand,
  assertValidSettingsLoadCommand,
  assertValidSettingsLoadOutcome,
  assertValidSettingsMutationResult,
  assertValidSettingsRegistrySnapshot,
  assertValidSettingsResetCommand,
  assertValidSettingsScopeSnapshot,
} from "../../src/settings/index.ts";
import { expectStrict } from "../corruption.ts";
import { fixture, registry } from "./support.ts";

// The consumer owns what is inside an opaque envelope.
const options = { raw: (path: string) => /\.value$/.test(path) };

function withModule(id: string): () => void {
  return () => {
    const value = registry();
    value.modules[0]!.id = id;
    assertValidSettingsRegistrySnapshot(value);
  };
}

describe("settings payload strictness", () => {
  test("refuses any corrupted member of the registry", () => {
    expectStrict(assertValidSettingsRegistrySnapshot, fixture.registry, options);
  });

  test("refuses any corrupted member of a scope payload or command", () => {
    const categories = [
      [assertValidSettingsScopeSnapshot, fixture.snapshots],
      [assertValidSettingsLoadCommand, fixture.loadCommands],
      [assertValidSettingsApplyCommand, fixture.applyCommands],
      [assertValidSettingsResetCommand, fixture.resetCommands],
      [assertValidSettingsLoadOutcome, fixture.loadOutcomes],
      [assertValidSettingsMutationResult, fixture.mutationResults],
    ] as const;
    for (const [validate, payloads] of categories) {
      for (const payload of payloads) {
        expectStrict(validate, payload, options);
      }
    }
  });

  test("refuses an unknown field on each module-owned definition", () => {
    for (const category of ["renderers", "scopes", "capabilities"] as const) {
      const value = registry();
      Object.assign(value[category][0]!, { extra: true });
      expect(() => assertValidSettingsRegistrySnapshot(value)).toThrow(
        "incompatible settings protocol: unknown_field",
      );
    }
  });

  test("admits exactly the identifiers Rust admits", () => {
    for (const id of ["_internal", ".hidden", ":scoped", "-dashed", "a".repeat(128)]) {
      expect(withModule(id)).not.toThrow();
    }
    for (const id of ["", "app/module", "App", "a".repeat(129)]) {
      expect(withModule(id)).toThrow("incompatible settings protocol: invalid_identity");
    }
  });
});
