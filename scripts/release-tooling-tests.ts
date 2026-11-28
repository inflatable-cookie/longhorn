// The `effigy test:release-tooling` runner. Its test files come from disk: every
// `*.test.ts` under `scripts/` is release tooling. A hand-kept list in effigy.toml
// let a new test file go unrun until someone remembered to add it.

import { readdir } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

export async function releaseToolingTestFiles(root = repoRoot): Promise<Array<string>> {
  const files: Array<string> = [];
  await collect(join(root, "scripts"), root, files);
  return files.sort();
}

async function collect(directory: string, root: string, files: Array<string>): Promise<void> {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const full = join(directory, entry.name);
    if (entry.isDirectory()) {
      await collect(full, root, files);
    } else if (entry.isFile() && entry.name.endsWith(".test.ts")) {
      files.push(relative(root, full));
    }
  }
}

if (import.meta.main) {
  const files = await releaseToolingTestFiles();
  if (files.length === 0) {
    throw new Error("no *.test.ts files found under scripts/");
  }
  const proc = Bun.spawn([process.execPath, "test", "--timeout=30000", "--no-orphans", ...files], {
    cwd: repoRoot,
    stdout: "inherit",
    stderr: "inherit",
  });
  process.exit(await proc.exited);
}
