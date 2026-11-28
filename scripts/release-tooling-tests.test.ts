import { afterEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { releaseToolingTestFiles } from "./release-tooling-tests.ts";

const repoRoot = resolve(import.meta.dir, "..");
const created: Array<string> = [];

afterEach(async () => {
  for (const dir of created.splice(0)) {
    await rm(dir, { recursive: true, force: true });
  }
});

describe("release tooling test list", () => {
  test("a test file nested under scripts/ is run without a list entry", async () => {
    const root = await mkdtemp(join(tmpdir(), "release-tooling-tests-"));
    created.push(root);
    await mkdir(join(root, "scripts", "nested"), { recursive: true });
    await writeFile(join(root, "scripts", "a.test.ts"), "");
    await writeFile(join(root, "scripts", "nested", "b.test.ts"), "");
    await writeFile(join(root, "scripts", "plain.ts"), "");
    expect(await releaseToolingTestFiles(root)).toEqual([
      "scripts/a.test.ts",
      "scripts/nested/b.test.ts",
    ]);
  });

  test("the repository's own test files are all listed", async () => {
    const files = await releaseToolingTestFiles(repoRoot);
    expect(files).toContain("scripts/check-runner-tools.test.ts");
    expect(files).toContain("scripts/bridge-topology-artifact-proof/artifacts.test.ts");
    expect(files).toContain("scripts/release-tooling-tests.test.ts");
  });
});
