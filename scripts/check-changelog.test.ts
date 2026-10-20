import { afterAll, describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { checkChangelog, judgeReleaseStatus } from "./check-changelog.ts";

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

describe("changelog check against Effigy's release parser", () => {
  test("passes a changelog with entries under [Unreleased]", async () => {
    const verdict = await checkChangelog(await fixture(withEntries));
    expect(verdict.outcome).toBe("pass");
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
    expect(verdict.outcome).toBe("pass");
  });
});

describe("judgeReleaseStatus", () => {
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
