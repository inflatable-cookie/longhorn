import { describe, expect, test } from "bun:test";
import { readdir, readFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

import { longhornVersion } from "./longhorn-version.ts";

const repoRoot = resolve(import.meta.dir, "..");

describe("version literals", () => {
  test("active proofs and boundary tests do not hardcode the workspace version", async () => {
    const version = longhornVersion(repoRoot);
    const needle = `"${version}"`;
    const files = [
      ...await collect(join(repoRoot, "scripts"), (name) => name.endsWith(".ts") && !name.endsWith(".test.ts")),
      join(repoRoot, "packages/longhorn-tauri/tests/boundary.test.ts"),
      join(repoRoot, "packages/longhorn-poodle-svelte/tests/boundary.test.ts"),
    ];
    const offenders: string[] = [];
    for (const file of files) {
      const text = await readFile(file, "utf8");
      if (text.includes(needle)) offenders.push(relative(repoRoot, file));
    }
    expect(offenders).toEqual([]);
  });
});

async function collect(dir: string, include: (name: string) => boolean): Promise<string[]> {
  const entries = await readdir(dir, { withFileTypes: true });
  const files: string[] = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...await collect(path, include));
      continue;
    }
    if (include(entry.name)) files.push(path);
  }
  return files;
}
