import { afterEach, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { ALLOWED_ESCAPES, verifyRepoContainment } from "./verify-repo-containment.ts";

const tempRoots: Array<string> = [];

afterEach(async () => {
  while (tempRoots.length > 0) {
    const root = tempRoots.pop();
    if (root) {
      await rm(root, { recursive: true, force: true });
    }
  }
});

async function fixtureRepo(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "longhorn-containment-test-"));
  tempRoots.push(root);
  await mkdir(join(root, "scripts"), { recursive: true });
  await mkdir(join(root, "crates", "sample"), { recursive: true });
  await mkdir(join(root, "packages", "sample"), { recursive: true });
  return root;
}

describe("verify-repo-containment", () => {
  test("passes on the live repository and no longer exempts verify-greenfield-compositions.ts", async () => {
    expect(ALLOWED_ESCAPES.has("scripts/verify-greenfield-compositions.ts")).toBe(false);
    await expect(verifyRepoContainment()).resolves.toMatchObject({
      schema: "longhorn.repo-containment.v1",
      outcome: "pass",
    });
  });

  test("catches an escape inside scripts/verify-greenfield-compositions.ts", async () => {
    const root = await fixtureRepo();
    const up = "..";
    await writeFile(
      join(root, "scripts", "verify-greenfield-compositions.ts"),
      `const poodle = resolve(repoRoot, "${up}/poodle");\n`,
    );

    await expect(verifyRepoContainment(root)).rejects.toThrow(
      /scripts\/verify-greenfield-compositions\.ts:1 resolves a path above the repository root/,
    );
  });

  test("catches Cargo path and package.json file:/link: escapes while keeping g02.036 admitted", async () => {
    const root = await fixtureRepo();
    await writeFile(
      join(root, "crates", "sample", "Cargo.toml"),
      '[dependencies]\nother = { path = "../../../other" }\n',
    );
    await writeFile(
      join(root, "packages", "sample", "package.json"),
      JSON.stringify({ dependencies: { other: "file:../../../other" } }),
    );
    const up = "..";
    await writeFile(
      join(root, "scripts", "verify-agent-tool-dispatch-source-consumer.ts"),
      `const sibling = resolve(repoRoot, "${up}/swallowtail");\n`,
    );

    await expect(verifyRepoContainment(root)).rejects.toThrow(
      /crates\/sample\/Cargo\.toml:2 Cargo path escapes[\s\S]*packages\/sample\/package\.json:1 file: dependency escapes/,
    );
  });
});
