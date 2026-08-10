import { describe, expect, test } from "bun:test";

import { assertCompatibleSettingsLoadCommand } from "../src/settings/compatibility.ts";
import { SETTINGS_FIELDS } from "../src/settings/generated/fields.ts";

/**
 * Contract 010's Boundary Validation Target on `settings` — 619 lines of
 * validation and zero key checks before this.
 */
describe("settings boundary strictness", () => {
  const valid = {
    protocolVersion: 1,
    requestId: "request:one",
    registryGeneration: 1,
    scopeId: "scope:one",
    knownAuthority: null,
  };

  test("the field map matches the fixture's keys", () => {
    expect([...SETTINGS_FIELDS.SettingsLoadCommand!].sort()).toEqual(
      Object.keys(valid).sort(),
    );
  });

  test("rejects an unknown field", () => {
    expect(() =>
      assertCompatibleSettingsLoadCommand({ ...valid, extra: 1 }),
    ).toThrow(/unknown_field|incompatible/);
  });

  test("rejects a missing field", () => {
    const { scopeId: _dropped, ...missing } = valid;
    expect(() => assertCompatibleSettingsLoadCommand(missing)).toThrow(
      /missing_field|incompatible/,
    );
  });

  test("omits tagged unions rather than guessing their keys", () => {
    expect(SETTINGS_FIELDS.SettingsLoadOutcome).toBeUndefined();
  });
});
