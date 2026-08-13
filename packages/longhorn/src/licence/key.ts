/**
 * The licence key format, in TypeScript.
 *
 * A second implementation of a format that already exists in Rust, which is
 * normally a mistake. It is here because Card 158 requires key entry to fail
 * locally: a mistyped key must never produce a message implying the key is
 * invalid, and telling those apart after a round trip is too late — the
 * authority answers `notRecognised`, which sends a customer to check their
 * purchase when they should check their typing.
 *
 * The two implementations are bound by `key-conformance.json`, generated from
 * the Rust and asserted against here. Neither can change the format without
 * the other's tests failing.
 *
 * This validates *shape*. Whether a well-formed key exists is the authority's
 * question and cannot be answered here.
 */

/** Crockford base32: no I, L, O, or U. */
const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/** The 32 symbols plus five check-only ones, giving the prime modulus 37. */
const CHECK_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ*~$=U";

/** Symbols per group in the display form. */
const GROUP = 5;

/**
 * The shortest key that may exist, check symbol included.
 *
 * Not a typing convenience. It is what makes distinguishing "not recognised"
 * from "revoked" safe: telling a caller a well-formed key is unknown lets them
 * learn which keys exist, and that only matters if keys are guessable.
 * Nineteen body symbols of Crockford base32 is ninety-five bits.
 */
export const MINIMUM_KEY_SYMBOLS = 20;

export type LicenceKeyProblem =
  /** A symbol outside the alphabet. Mistyped. */
  | { readonly kind: "unexpectedSymbol"; readonly symbol: string }
  /** Too few symbols to be a key. Not a typo — a truncation or a wrong value. */
  | { readonly kind: "tooShort"; readonly minimum: number; readonly actual: number }
  /** The check symbol disagrees with the body. Mistyped. */
  | { readonly kind: "checkFailed" };

export type LicenceKeyResult =
  | { readonly ok: true; readonly key: string; readonly grouped: string }
  | { readonly ok: false; readonly problem: LicenceKeyProblem };

/**
 * Whether a problem means the customer mistyped.
 *
 * The distinction the whole file exists for. A surface that cannot tell these
 * apart says "invalid licence key" to someone who transposed two characters,
 * and they believe they were sold a dud.
 */
export function isProbablyATypo(problem: LicenceKeyProblem): boolean {
  return problem.kind === "checkFailed" || problem.kind === "unexpectedSymbol";
}

/**
 * Parses a key as typed, accepting anything a person plausibly enters.
 *
 * Lower case, missing dashes, extra dashes, and surrounding or embedded
 * whitespace are all accepted. So are the classic confusions: `I` and `L` read
 * as `1`, `O` reads as `0`. Rejecting these would be rejecting the customer
 * for the typeface's mistake.
 */
export function parseLicenceKey(input: string): LicenceKeyResult {
  const normalized = normalize(input);
  if (!normalized.ok) return normalized;

  const value = normalized.value;
  if (value.length < MINIMUM_KEY_SYMBOLS) {
    return {
      ok: false,
      problem: { kind: "tooShort", minimum: MINIMUM_KEY_SYMBOLS, actual: value.length },
    };
  }

  const body = value.slice(0, -1);
  if (value.slice(-1) !== checkSymbol(body)) {
    return { ok: false, problem: { kind: "checkFailed" } };
  }
  return { ok: true, key: value, grouped: group(value) };
}

/** Renders a normalized key as `XXXXX-XXXXX-…`. */
export function group(key: string): string {
  const groups: string[] = [];
  for (let at = 0; at < key.length; at += GROUP) groups.push(key.slice(at, at + GROUP));
  return groups.join("-");
}

function normalize(
  input: string,
): { readonly ok: true; readonly value: string } | { readonly ok: false; readonly problem: LicenceKeyProblem } {
  let output = "";
  for (const character of input) {
    if (character === "-" || /\s/u.test(character)) continue;
    const upper = character.toUpperCase();
    // The confusions Crockford excluded the symbols to avoid.
    const mapped = upper === "I" || upper === "L" ? "1" : upper === "O" ? "0" : upper;
    if (!CHECK_ALPHABET.includes(mapped) || mapped.length !== 1) {
      return { ok: false, problem: { kind: "unexpectedSymbol", symbol: character } };
    }
    output += mapped;
  }
  return { ok: true, value: output };
}

/**
 * The position-weighted check symbol for a body.
 *
 * Weighted by position on purpose: an unweighted sum accepts any reordering,
 * and transposition is one of the two mistakes people actually make.
 */
function checkSymbol(body: string): string {
  let sum = 0;
  for (let index = 0; index < body.length; index += 1) {
    const value = ALPHABET.indexOf(body[index]!);
    sum += (value === -1 ? 0 : value) * (index + 1);
  }
  return CHECK_ALPHABET[sum % 37]!;
}
