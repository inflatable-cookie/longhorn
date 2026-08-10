import { describe, expect, test } from "bun:test";

import { assertCompatibleCommandCatalogueChangedEvent } from "../src/commands/validation.ts";
import { COMMANDS_FIELDS } from "../src/commands/generated/fields.ts";

/**
 * Contract 010's Boundary Validation Target on `commands` — 387 lines of
 * validation, twelve Rust `MAXIMUM_*` constants, and no key checking at all
 * before this.
 */
describe("commands boundary strictness", () => {
  const valid = {
    protocolVersion: 1,
    registryGeneration: 1,
  };

  test("the field map matches the fixture's keys", () => {
    expect([...COMMANDS_FIELDS.CommandCatalogueChangedEvent!].sort()).toEqual(
      Object.keys(valid).sort(),
    );
  });

  test("rejects an unknown field", () => {
    expect(() =>
      assertCompatibleCommandCatalogueChangedEvent({ ...valid, extra: 1 }),
    ).toThrow(/unknown field/);
  });

  test("rejects a missing field", () => {
    const { registryGeneration: _dropped, ...missing } = valid;
    expect(() =>
      assertCompatibleCommandCatalogueChangedEvent(missing),
    ).toThrow(/missing field/);
  });

  test("omits tagged unions rather than guessing their keys", () => {
    expect(COMMANDS_FIELDS.CommandKeymapLoadOutcome).toBeUndefined();
  });
});
