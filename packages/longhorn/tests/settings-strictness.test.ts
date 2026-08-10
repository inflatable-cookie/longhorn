import { describe, expect, test } from "bun:test";

import { assertCompatibleSettingsLoadCommand } from "../src/settings/validation.ts";
import { SETTINGS_FIELDS } from "../src/settings/generated/fields.ts";
import fixture from "../../../fixtures/settings/protocol-v1.json";

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

  /**
   * `ts-rs` separates fields with a comma when it renders a type across lines
   * and a semicolon when it renders one inline. The field-map generator split
   * on commas only, so `SettingsOpaqueValue` — the one type in the repository
   * emitted inline — came out as `["codecVersion"]` with `value` dropped.
   *
   * A short list is the dangerous direction: it rejects payloads that are
   * valid, and it does so at the boundary rather than failing in the
   * generator. Three fixture categories broke at once.
   */
  test("keeps both fields of an inline-rendered type", () => {
    expect(SETTINGS_FIELDS.SettingsOpaqueValue).toEqual([
      "codecVersion",
      "value",
    ]);
    expect(Object.keys(fixture.recoveryStates[0]!.diagnostic).sort()).toEqual([
      "codecVersion",
      "value",
    ]);
  });
});
