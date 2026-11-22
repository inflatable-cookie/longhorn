import { describe, expect, test } from "bun:test";

import { assertValidCommandCatalogueChangedEvent } from "../../src/commands/validation.ts";
import { COMMANDS_FIELDS } from "../../src/commands/generated/fields.ts";

/**
 * Boundary validation for `commands`, against contract 010's Boundary
 * Validation Target: unknown keys are rejected, and each bounded field is
 * checked against its Rust `MAXIMUM_*` constant.
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
      assertValidCommandCatalogueChangedEvent({ ...valid, extra: 1 }),
    ).toThrow(/unknown field/);
  });

  test("rejects a missing field", () => {
    const { registryGeneration: _dropped, ...missing } = valid;
    expect(() =>
      assertValidCommandCatalogueChangedEvent(missing),
    ).toThrow(/missing field/);
  });

  test("omits tagged unions rather than guessing their keys", () => {
    expect(COMMANDS_FIELDS.CommandKeymapLoadOutcome).toBeUndefined();
  });
});
