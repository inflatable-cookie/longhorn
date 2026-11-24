// Attention markers, the current form of the retired `[scan.attention_markers]`
// table.
//
// Effigy 0.15 no longer scans markers, and its retirement warning points at a
// plain search. A search is not a gate, so the critical tier stays one here:
// `BUG` and `remove before release` in tracked files fail `check:markers`.
// Matching is case-sensitive and whole-word. The retired scanner matched
// case-insensitive substrings, so `debug` and prose `bug` flagged 2,248 lines
// in this repository, which is noise rather than a marker. The high and warning
// tiers are reported as counts and never fail.

import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

export const CRITICAL_MARKERS: ReadonlyArray<string> = ["BUG", "remove before release"];
export const HIGH_MARKERS: ReadonlyArray<string> = ["FIXME", "HACK", "@deprecated", "workaround"];
export const WARNING_MARKERS: ReadonlyArray<string> = ["TODO", "REVIEW", "NOTE", "placeholder"];

// Files that name the markers as data rather than carrying them.
const MARKER_DATA_EXCLUDES = [
  ":(exclude)effigy.toml",
  ":(exclude)scripts/check-markers.ts",
  ":(exclude)scripts/check-markers.test.ts",
];

export type MarkerHit = {
  readonly path: string;
  readonly line: number;
  readonly text: string;
};

// git grep arguments for a set of fixed, whole-word, case-sensitive markers
// over tracked files.
export function markerGrepArgs(markers: ReadonlyArray<string>): Array<string> {
  const args = ["grep", "-n", "-w", "-F"];
  for (const marker of markers) {
    args.push("-e", marker);
  }
  return [...args, "--", ".", ...MARKER_DATA_EXCLUDES];
}

// Parses `path:line:text` lines from git grep -n. The text may contain colons.
export function parseGitGrep(stdout: string): Array<MarkerHit> {
  const hits: Array<MarkerHit> = [];
  for (const line of stdout.split("\n")) {
    if (line === "") {
      continue;
    }
    const match = /^(.+?):(\d+):(.*)$/.exec(line);
    if (match === null) {
      throw new Error(`unexpected git grep output: ${line}`);
    }
    hits.push({
      path: match[1] ?? "",
      line: Number(match[2]),
      text: match[3] ?? "",
    });
  }
  return hits;
}

export function criticalFailures(hits: ReadonlyArray<MarkerHit>): Array<string> {
  return hits.map(
    (hit) => `${hit.path}:${hit.line}: critical attention marker: ${hit.text.trim()}`,
  );
}

function grepMarkers(markers: ReadonlyArray<string>): Array<MarkerHit> {
  const result = spawnSync("git", markerGrepArgs(markers), {
    cwd: repoRoot,
    encoding: "utf8",
  });
  // git grep exits 1 when nothing matches; anything else other than 0 is an error.
  if (result.status === 1) {
    return [];
  }
  if (result.status !== 0) {
    throw new Error(`git grep failed: ${result.stderr || String(result.error)}`);
  }
  return parseGitGrep(result.stdout);
}

if (import.meta.main) {
  const failures = criticalFailures(grepMarkers(CRITICAL_MARKERS));
  const high = grepMarkers(HIGH_MARKERS).length;
  const warning = grepMarkers(WARNING_MARKERS).length;
  if (failures.length > 0) {
    for (const failure of failures) {
      console.error(`check:markers: ${failure}`);
    }
    process.exit(1);
  }
  console.log(
    `check:markers: no critical markers; high ${high} and warning ${warning} matching lines (report only)`,
  );
}
