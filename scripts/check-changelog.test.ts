import { afterAll, describe, expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

import { checkChangelog, judgeReleaseStatus } from "./check-changelog.ts";
import type { ChangelogVerdict } from "./check-changelog.ts";

// Each fixture is a fresh mktemp repo with only what Effigy's release status
// reads: the workspace version, the release table, and the changelog.
const cargoToml = `[workspace]
members = []

[workspace.package]
version = "0.3.3"
`;

const effigyToml = `[release]
version-file = "Cargo.toml"
version-path = "workspace.package.version"
changelog = "CHANGELOG.md"
pre-1-0 = true
tag-format = "v{version}"
`;

const scratchRoots: Array<string> = [];

async function fixture(changelog: string): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "check-changelog-"));
  scratchRoots.push(root);
  await writeFile(join(root, "Cargo.toml"), cargoToml);
  await writeFile(join(root, "effigy.toml"), effigyToml);
  await writeFile(join(root, "CHANGELOG.md"), changelog);
  return root;
}

const withEntries = `# Changelog

## [Unreleased]

### Added
- Something new.

## [0.3.3] - 2026-10-19

### Added
- Something shipped.
`;

const emptyUnreleased = `# Changelog

## [Unreleased]

## [0.3.3] - 2026-10-19

### Added
- Something shipped.
`;

const releaseFacts = `# Changelog

## [Unreleased]

## [0.3.3] - 2026-10-19

### Release facts

- Something that is not a category.
`;

const malformedHeading = `# Changelog

## [Unreleased]

## 0.3.3 - 2026-10-19

### Added
- Something shipped.
`;

afterAll(async () => {
  for (const root of scratchRoots) {
    await rm(root, { recursive: true, force: true });
  }
});

// A bare `expect(outcome).toBe("pass")` hides why Effigy refused the file, and
// the first CI failure on the released 0.14.1 explained nothing (longhorn#063).
function expectPass(verdict: ChangelogVerdict): void {
  if (verdict.outcome === "fail") {
    throw new Error(`Effigy refused CHANGELOG.md: ${verdict.reason}`);
  }
}

describe("changelog check against Effigy's release parser", () => {
  test("passes a changelog with entries under [Unreleased]", async () => {
    const verdict = await checkChangelog(await fixture(withEntries));
    expectPass(verdict);
  });

  test("passes an empty [Unreleased], the normal state after a release", async () => {
    const verdict = await checkChangelog(await fixture(emptyUnreleased));
    expect(verdict).toEqual({
      outcome: "pass",
      note: "CHANGELOG.md parses; [Unreleased] is empty",
    });
  });

  test("fails a Release facts category, the longhorn#049 case", async () => {
    const verdict = await checkChangelog(await fixture(releaseFacts));
    expect(verdict.outcome).toBe("fail");
    expect(verdict.outcome === "fail" && verdict.reason).toContain(
      "unknown category `Release facts`",
    );
  });

  test("fails a release heading without brackets", async () => {
    const verdict = await checkChangelog(await fixture(malformedHeading));
    expect(verdict.outcome).toBe("fail");
    expect(verdict.outcome === "fail" && verdict.reason).toContain("invalid release header");
  });

  test("stages a clean copy so checkout release journals and config/release.toml do not apply", async () => {
    const root = await fixture(withEntries);
    await mkdir(join(root, "config"));
    await writeFile(join(root, "config/release.toml"), effigyToml);
    await writeFile(join(root, "effigy.toml"), '[manifest]\ninclude = ["config/release.toml"]\n');
    await mkdir(join(root, ".effigy"));
    await writeFile(join(root, ".effigy/release-state.json"), "{not-valid-json");

    const verdict = await checkChangelog(root);
    expectPass(verdict);
  });

  test("reads the binary from EFFIGY_BIN and runs it inside the staged directory", async () => {
    const binDir = await mkdtemp(join(tmpdir(), "check-changelog-bin-"));
    scratchRoots.push(binDir);
    const cwdCapture = join(binDir, "cwd");
    const bin = join(binDir, "effigy");
    await writeFile(
      bin,
      `#!/bin/sh\ntest -f Cargo.toml && test -f effigy.toml && test -f CHANGELOG.md || exit 2\npwd -P > '${cwdCapture}'\nprintf '%s\\n' '{"ok":true}'\n`,
    );
    await chmod(bin, 0o755);
    const repoFixture = await fixture(withEntries);
    const previous = process.env.EFFIGY_BIN;
    process.env.EFFIGY_BIN = bin;
    try {
      const verdict = await checkChangelog(repoFixture);
      expect(verdict).toEqual({
        outcome: "pass",
        note: "Effigy's release parser accepts CHANGELOG.md",
      });
      const observedCwd = (await readFile(cwdCapture, "utf8")).trim();
      expect(observedCwd).not.toBe(await realpath(repoFixture));
      expect(basename(observedCwd)).toStartWith("check-changelog-stage-");
    } finally {
      if (previous === undefined) {
        delete process.env.EFFIGY_BIN;
      } else {
        process.env.EFFIGY_BIN = previous;
      }
    }
  });
});

describe("judgeReleaseStatus", () => {
  test("accepts the empty [Unreleased] envelope released 0.14.1 prints", () => {
    const stdout = JSON.stringify({
      binary: {
        active_version: "0.14.1",
        display_version: "v0.14.1",
        name: "effigy",
        version: "0.14.1",
      },
      command: { kind: "release", name: "release" },
      ok: false,
      result: null,
      schema: "effigy.command.v1",
      schema_version: 1,
      error: {
        kind: "RunnerError",
        message: "command failed (json output available)",
        details: {
          blockers: ["unreleased changelog section has no entries"],
          changelog: { diagnostic_count: 0, diagnostics: [], path: "CHANGELOG.md", valid: true },
          current_version: "0.3.4",
          gate_check_passed: false,
          gates: { checked: false, configured_count: 0, results: [] },
          gates_passed: false,
          next_version: null,
          ready: false,
          schema: "effigy.release.status.v1",
          schema_version: 1,
          suggested_bump: "none",
          tag: null,
          unreleased: { counts: {}, empty: true, entry_count: 0 },
        },
      },
    });
    expect(judgeReleaseStatus(stdout)).toEqual({
      outcome: "pass",
      note: "CHANGELOG.md parses; [Unreleased] is empty",
    });
  });

  test("passes when release run state is unreadable but the changelog parses", () => {
    // Reproduced shape (longhorn#174): the staged copy sits under a checkout
    // that carries an Effigy release journal, so `release status` refuses the
    // Run while its `details` still report the changelog parse.
    const stdout = JSON.stringify({
      ok: false,
      error: {
        kind: "RunnerError",
        message: "command failed (json output available)",
        details: {
          blockers: [
            "unreleased changelog section has no entries",
            "release Run state is unreadable: release checkout state: journal exists without its checkout identity",
          ],
          changelog: { diagnostic_count: 0, diagnostics: [], path: "CHANGELOG.md", valid: true },
          unreleased: { counts: {}, empty: true, entry_count: 0 },
        },
      },
    });
    expect(judgeReleaseStatus(stdout)).toEqual({
      outcome: "pass",
      note: "CHANGELOG.md parses; [Unreleased] is empty. Effigy also refused release run state outside this changelog: release Run state is unreadable: release checkout state: journal exists without its checkout identity",
    });
  });

  test("does not read a non-changelog runner failure as the empty gap", () => {
    const stdout = JSON.stringify({
      ok: false,
      error: {
        kind: "RunnerError",
        message: "command failed (json output available)",
        details: {
          blockers: ["unreleased changelog section has no entries", "version file is missing"],
          changelog: { valid: true },
          unreleased: { empty: true },
        },
      },
    });
    expect(judgeReleaseStatus(stdout).outcome).toBe("fail");
  });

  // A refused status that names no cause cannot vouch for the changelog. The
  // empty-section gap always arrives as a blocker, so `every()` over an absent,
  // empty or non-string list is not a pass (review of longhorn#174).
  test.each([
    ["absent", undefined],
    ["empty", []],
    ["non-string", [{ message: "gates failed" }]],
  ])("fails %s blockers even when the changelog parsed", (_case, blockers) => {
    const parsed = {
      changelog: { diagnostic_count: 0, diagnostics: [], path: "CHANGELOG.md", valid: true },
      unreleased: { counts: {}, empty: true, entry_count: 0 },
    };
    const details = blockers === undefined ? parsed : { ...parsed, blockers };
    const stdout = JSON.stringify({
      ok: false,
      error: {
        kind: "RunnerError",
        message: "command failed (json output available)",
        details,
      },
    });
    const verdict = judgeReleaseStatus(stdout);
    expect(verdict.outcome).toBe("fail");
    expect(verdict.outcome === "fail" && verdict.reason).toContain("no blocker named");
  });

  test("does not read a parse error without details as the empty gap", () => {
    const stdout = JSON.stringify({
      ok: false,
      error: {
        kind: "RunnerError",
        message: "changelog parse errors:\n  line 7: unknown category `Release facts`",
        details: null,
      },
    });
    expect(judgeReleaseStatus(stdout).outcome).toBe("fail");
  });

  test("fails output that is not JSON", () => {
    expect(judgeReleaseStatus("Release Status\n").outcome).toBe("fail");
  });
});
