import { describe, expect, test } from "bun:test";

import { assertValidConfigOperationsSnapshot } from "../src/config/validation.ts";
import { CONFIG_FIELDS } from "../src/config/generated/fields.ts";

/**
 * Contract 010's Boundary Validation Target on `config` — the package the
 * inventory named first for having 858 lines of validation and zero key
 * checks. A renamed field passed straight through.
 */
describe("config boundary strictness", () => {
  test("the snapshot's allowed keys come from the authority", () => {
    expect(CONFIG_FIELDS.ConfigOperationsSnapshot).toBeDefined();
    expect(CONFIG_FIELDS.ConfigOperationsSnapshot).toContain("protocolVersion");
  });

  test("rejects an unknown field", () => {
    const snapshot: Record<string, unknown> = {};
    for (const key of CONFIG_FIELDS.ConfigOperationsSnapshot!) {
      snapshot[key] = null;
    }
    expect(() =>
      assertValidConfigOperationsSnapshot({ ...snapshot, extra: 1 }),
    ).toThrow(/unknown field/);
  });

  test("rejects a missing field", () => {
    const snapshot: Record<string, unknown> = {};
    for (const key of CONFIG_FIELDS.ConfigOperationsSnapshot!.slice(1)) {
      snapshot[key] = null;
    }
    expect(() => assertValidConfigOperationsSnapshot(snapshot)).toThrow(
      /missing field/,
    );
  });

  test("omits tagged unions rather than guessing their keys", () => {
    expect(CONFIG_FIELDS.RestoreDomainCompatibilityProjection).toBeUndefined();
  });
});
