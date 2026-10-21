import { afterAll, describe, expect, test } from "bun:test";
import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { checkChangelog, type ChangelogVerdict, judgeReleaseStatus } from "./check-changelog.ts";

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

const scratchRoots: string[] = [];

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

  test("passes the repository's own CHANGELOG.md", async () => {
    const verdict = await checkChangelog();
    expectPass(verdict);
  });

  test("reads the binary from EFFIGY_BIN, so the released version can be checked", async () => {
    const binDir = await mkdtemp(join(tmpdir(), "check-changelog-bin-"));
    scratchRoots.push(binDir);
    const bin = join(binDir, "effigy");
    await writeFile(bin, `#!/bin/sh\nprintf '%s\\n' '{"ok":true}'\n`);
    await chmod(bin, 0o755);
    const previous = process.env.EFFIGY_BIN;
    process.env.EFFIGY_BIN = bin;
    try {
      const verdict = await checkChangelog(await fixture(withEntries));
      expect(verdict).toEqual({
        outcome: "pass",
        note: "Effigy's release parser accepts CHANGELOG.md",
      });
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
