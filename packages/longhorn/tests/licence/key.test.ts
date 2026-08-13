import { describe, expect, test } from "bun:test";
import cases from "../../src/licence/generated/key-conformance.json";
import {
  MINIMUM_KEY_SYMBOLS,
  isProbablyATypo,
  parseLicenceKey,
  type LicenceKeyProblem,
} from "../../src/licence/key.ts";

/** The Rust outcome vocabulary, rendered from a TypeScript result. */
function outcome(input: string): string {
  const result = parseLicenceKey(input);
  if (result.ok) return `ok:${result.grouped}`;
  const problem: LicenceKeyProblem = result.problem;
  switch (problem.kind) {
    case "checkFailed":
      return "checkFailed";
    case "tooShort":
      return `tooShort:${problem.minimum}:${problem.actual}`;
    case "unexpectedSymbol":
      return `unexpectedSymbol:${problem.symbol}`;
  }
}

describe("licence key format", () => {
  /**
   * The reason a second implementation is tolerable at all.
   *
   * Two implementations of one format drift, and the only defence is a fixture
   * neither side authors. This one is generated from `LicenceKey` in Rust, so
   * changing the format on either side fails here.
   */
  test("agrees with the Rust implementation on every generated case", () => {
    expect(cases.length).toBeGreaterThan(8);
    for (const { input, outcome: expected } of cases) {
      expect(outcome(input)).toBe(expected);
    }
  });

  /**
   * Card 158's acceptance criterion, and the whole point of validating
   * locally: a mistyped key must never read as an invalid one. `checkFailed`
   * says check your typing; `tooShort` says this is not a key at all.
   */
  test("a mistyped key is distinguishable from a key that is not one", () => {
    const mistyped = parseLicenceKey("ABCDE12345FGHJK6789Z");
    const truncated = parseLicenceKey("ABCDE12345");

    expect(mistyped.ok).toBeFalse();
    expect(truncated.ok).toBeFalse();
    if (mistyped.ok || truncated.ok) throw new Error("expected both to fail");

    expect(isProbablyATypo(mistyped.problem)).toBeTrue();
    expect(isProbablyATypo(truncated.problem)).toBeFalse();
  });

  test("the entropy floor is stated once and enforced", () => {
    expect(MINIMUM_KEY_SYMBOLS).toBe(20);
    const short = parseLicenceKey("ABCDE");
    expect(short.ok).toBeFalse();
    if (!short.ok) expect(short.problem).toEqual({ kind: "tooShort", minimum: 20, actual: 5 });
  });
});
