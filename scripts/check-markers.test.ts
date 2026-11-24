import { describe, expect, test } from "bun:test";

import {
  ATTENTION_PATTERNS,
  attentionCategory,
  attentionFileFindings,
  isAttentionScanPath,
  isGeneratedArtifact,
} from "./check-markers.ts";

// `marker:severity:category` for each finding in one file, in order.
function summary(path: string, text: string): Array<string> {
  return attentionFileFindings(path, text).map(
    (f) => `${f.line}:${f.marker}:${f.severity}:${f.category}`,
  );
}

describe("the marker lists", () => {
  test("are the lists the retired table set", () => {
    expect(ATTENTION_PATTERNS).toEqual({
      critical: ["BUG", "remove before release"],
      high: ["FIXME", "HACK", "@deprecated", "workaround"],
      warning: ["TODO", "REVIEW", "NOTE", "placeholder"],
    });
  });
});

describe("rule 1: scope", () => {
  test("code files outside the excluded, documentation, data and lockfile paths are scanned", () => {
    for (const path of [
      "src/x.ts",
      "crates/x/src/lib.rs",
      "prototypes/x/src/a.rs",
      "scripts/x.sh",
      "app/x.zsh",
      "packages/p/src/a.vue",
    ]) {
      expect(isAttentionScanPath(path)).toBe(true);
    }
  });

  test("documentation, data, excluded directories, lockfiles, non-code and dot-only names are not", () => {
    for (const path of [
      "docs/x.ts",
      "examples/x.ts",
      "fixtures/x.ts",
      "benchmarks/x.rs",
      "README.md",
      "Cargo.lock",
      "bun.lock",
      "node_modules/a/x.ts",
      "build/x.ts",
      "target/x.rs",
      ".github/workflows/ci.yml",
      "packages/p/src/a.svelte",
      "src/.ts",
    ]) {
      expect(isAttentionScanPath(path)).toBe(false);
    }
  });

  test("an out-of-scope file yields no findings even with a marker", () => {
    expect(summary("docs/x.ts", "// BUG: broken\n")).toEqual([]);
  });
});

describe("rule 2: generated artifacts are skipped", () => {
  test("a .min. name, a .generated.rs name and a generated header are skipped", () => {
    expect(isGeneratedArtifact("src/a.min.js", "x\n")).toBe(true);
    expect(isGeneratedArtifact("src/a.generated.rs", "x\n")).toBe(true);
    expect(isGeneratedArtifact("src/a.ts", "// do not edit\n")).toBe(true);
    expect(summary("src/a.min.js", "// BUG: x\n")).toEqual([]);
    expect(summary("src/a.ts", "// do not edit\n// BUG: x\n")).toEqual([]);
  });

  test("a source map is not a generated artifact for this rule", () => {
    expect(isGeneratedArtifact("src/app.js.map", "x\n")).toBe(false);
  });
});

describe("rule 3: a marker starts a comment body", () => {
  test("a comment body that starts with the marker matches, in any case", () => {
    expect(summary("src/a.ts", "// BUG: wrong key order\n")).toEqual([
      "1:BUG:critical:deferred-work",
    ]);
    expect(summary("src/a.ts", "// bug: lower case\n")).toEqual(["1:BUG:critical:deferred-work"]);
  });

  test("a marker that is only a prefix of a longer word does not match", () => {
    expect(summary("src/a.ts", "// BUGS to fix\n")).toEqual([]);
    expect(summary("src/a.ts", "// BUGFIX\n")).toEqual([]);
    expect(summary("src/a.ts", "// BUG_TYPE = 1\n")).toEqual([]);
  });

  test("a marker in prose after other words does not match", () => {
    expect(summary("src/a.ts", "// a bug here\n")).toEqual([]);
    expect(summary("src/a.ts", "// debug logging\n")).toEqual([]);
  });

  test("a marker followed by end, colon, space, bracket, parenthesis or hyphen matches", () => {
    expect(summary("src/a.ts", "// BUG\n")).toHaveLength(1);
    expect(summary("src/a.ts", "// BUG-1234\n")).toHaveLength(1);
    expect(summary("src/a.ts", "// [BUG] x\n")).toHaveLength(1);
    expect(summary("src/a.ts", "// (BUG) x\n")).toHaveLength(1);
  });

  test("an inline comment body counts when the marker starts it", () => {
    expect(summary("src/a.ts", "x = 1 // BUG\n")).toEqual(["1:BUG:critical:deferred-work"]);
    expect(summary("src/a.ts", "x = 1 // trailing BUG\n")).toEqual([]);
  });

  test("bullets and brackets before the marker are skipped", () => {
    expect(summary("src/a.ts", "/* - BUG: x */\n")).toEqual(["1:BUG:critical:deferred-work"]);
  });

  test("a hash comment matches, but a Rust attribute does not", () => {
    expect(summary("src/a.py", "# TODO python\n")).toEqual(["1:TODO:warning:deferred-work"]);
    expect(summary("src/a.rs", "#[ TODO ]\n")).toEqual([]);
  });

  test("a marker in a string literal is masked and does not match", () => {
    expect(summary("src/a.ts", '// "BUG" quoted\n')).toEqual([]);
    expect(summary("src/a.ts", 'let s = "// BUG";\n')).toEqual([]);
  });

  test("a marker with no comment prefix does not match", () => {
    expect(summary("src/a.ts", "plain BUG without a comment\n")).toEqual([]);
  });

  test("a block-comment marker matches after the opening", () => {
    expect(summary("src/a.ts", "/* FIXME */\n")).toEqual(["1:FIXME:high:deferred-work"]);
  });
});

describe("rule 4: one finding per marker per line", () => {
  test("a repeated marker on one line counts once", () => {
    expect(summary("src/a.ts", "// @deprecated and @deprecated\n")).toEqual([
      "1:@deprecated:high:deprecation",
    ]);
  });

  test("a second marker not at the comment start is not reported", () => {
    expect(summary("src/a.ts", "// BUG and TODO on one line\n")).toEqual([
      "1:BUG:critical:deferred-work",
    ]);
  });

  test("a block-comment marker and an inline comment marker are both reported, in severity order", () => {
    expect(summary("src/a.ts", "/* BUG */ // TODO\n")).toEqual([
      "1:BUG:critical:deferred-work",
      "1:TODO:warning:deferred-work",
    ]);
  });
});

describe("rule 5: categories", () => {
  test("deprecation for a deprecation marker", () => {
    expect(attentionCategory("@deprecated")).toBe("deprecation");
    expect(attentionCategory("deprecated")).toBe("deprecation");
  });

  test("temporary artifact for a temporary word", () => {
    expect(attentionCategory("remove before release")).toBe("temporary-artifact");
    expect(attentionCategory("workaround")).toBe("temporary-artifact");
    expect(attentionCategory("placeholder")).toBe("temporary-artifact");
    expect(attentionCategory("stub")).toBe("temporary-artifact");
    expect(attentionCategory("tech debt")).toBe("temporary-artifact");
  });

  test("deferred work for everything else", () => {
    expect(attentionCategory("TODO")).toBe("deferred-work");
    expect(attentionCategory("FIXME")).toBe("deferred-work");
    expect(attentionCategory("BUG")).toBe("deferred-work");
  });

  test("a finding carries its category", () => {
    expect(summary("src/a.ts", "// remove before release\n")).toEqual([
      "1:remove before release:critical:temporary-artifact",
    ]);
    expect(summary("src/a.ts", "// workaround here\n")).toEqual([
      "1:workaround:high:temporary-artifact",
    ]);
    expect(summary("src/a.ts", "// @deprecated thing\n")).toEqual([
      "1:@deprecated:high:deprecation",
    ]);
  });
});

describe("rule 6: severity comes from the list", () => {
  test("critical, high and warning markers are each reported at their tier", () => {
    expect(summary("src/a.ts", "// BUG\n// HACK\n// NOTE\n")).toEqual([
      "1:BUG:critical:deferred-work",
      "2:HACK:high:deferred-work",
      "3:NOTE:warning:deferred-work",
    ]);
  });

  test("a word that is not in a list is not reported", () => {
    expect(summary("src/a.ts", "// deprecated thing\n")).toEqual([]);
  });
});

describe("line and snippet", () => {
  test("lines are 1-based and the snippet is the trimmed line, capped at 120 characters", () => {
    const long = `// BUG ${"x".repeat(200)}`;
    const [finding] = attentionFileFindings("src/a.ts", `first\n${long}\n`);
    expect(finding?.line).toBe(2);
    expect(finding?.snippet).toHaveLength(120);
    expect(finding?.snippet.startsWith("// BUG")).toBe(true);
  });
});
