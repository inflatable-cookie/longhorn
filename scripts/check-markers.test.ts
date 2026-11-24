import { describe, expect, test } from "bun:test";

import {
  CRITICAL_MARKERS,
  criticalFailures,
  markerGrepArgs,
  parseGitGrep,
} from "./check-markers.ts";

describe("markerGrepArgs", () => {
  test("matches fixed, whole-word, case-sensitive markers over tracked files", () => {
    const args = markerGrepArgs(CRITICAL_MARKERS);
    expect(args.slice(0, 4)).toEqual(["grep", "-n", "-w", "-F"]);
    expect(args).toContain("BUG");
    expect(args).toContain("remove before release");
    expect(args).not.toContain("-i");
  });

  test("excludes the files that name the markers as data", () => {
    const args = markerGrepArgs(CRITICAL_MARKERS);
    expect(args).toContain(":(exclude)effigy.toml");
    expect(args).toContain(":(exclude)scripts/check-markers.ts");
  });
});

describe("parseGitGrep", () => {
  test("reads path, line and text, keeping colons inside the text", () => {
    const hits = parseGitGrep('packages/a/src/b.ts:12:const s = "x:1:y"; // BUG\n');
    expect(hits).toEqual([
      { path: "packages/a/src/b.ts", line: 12, text: 'const s = "x:1:y"; // BUG' },
    ]);
  });

  test("returns no hits for empty output", () => {
    expect(parseGitGrep("")).toEqual([]);
  });

  test("refuses a line without a path and line number", () => {
    expect(() => parseGitGrep("not a grep line")).toThrow("unexpected git grep output");
  });
});

describe("criticalFailures", () => {
  test("a planted uppercase BUG marker is a failure", () => {
    const hits = parseGitGrep("crates/x/src/lib.rs:7:    // BUG: wrong key order\n");
    expect(criticalFailures(hits)).toEqual([
      "crates/x/src/lib.rs:7: critical attention marker: // BUG: wrong key order",
    ]);
  });

  test("a planted 'remove before release' marker is a failure", () => {
    const hits = parseGitGrep("scripts/x.ts:3:// remove before release\n");
    expect(criticalFailures(hits)).toHaveLength(1);
  });

  test("no hits means no failures", () => {
    expect(criticalFailures([])).toEqual([]);
  });
});
