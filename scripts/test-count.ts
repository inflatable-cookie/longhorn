// Reading a test count out of vitest's summary line.
//
// Five proofs each had their own copy of this, all matching
// `/Tests\s+(\d+) passed/` against raw captured output. That works locally and
// returns 0 on a runner, because the summary arrives coloured:
//
//     \x1b[2m Tests \x1b[22m \x1b[1m\x1b[32m9 passed\x1b[39m\x1b[22m
//
// Nothing here asked for colour. Vitest colours through picocolors, which
// enables it whenever `CI` is set, with no terminal involved -- so piping the
// output is not enough to get plain text, and GitHub Actions always sets `CI`.
// A proof then sees zero tests and reports a count mismatch, naming the
// consumer rather than the parse, which is a long way from the cause.
//
// Reproduce locally with `CI=1 bun scripts/verify-command-system-artifacts.ts`.
//
// Escapes are stripped rather than suppressed at the source: these proofs
// spawn vitest from several places, and one that forgets `NO_COLOR` would
// bring the bug back silently.

// eslint-disable-next-line no-control-regex
const ANSI = /\x1b\[[0-9;]*m/g;

/**
 * Returns the number of passing tests in vitest summary output, or 0 when the
 * summary is absent.
 *
 * Callers treat 0 as a failure, which is the right reading: a proof that
 * mounts a consumer and runs its suite expects a positive count, and both an
 * empty suite and an unparseable summary mean the proof did not observe what
 * it claims to.
 */
export function testCount(output: string): number {
  const match = output.replace(ANSI, "").match(/Tests\s+(\d+) passed/);
  return match ? Number(match[1]) : 0;
}
