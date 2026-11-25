// Test-only scratch bookkeeping for the release-tooling suite.
//
// A `mkdtemp` fixture that is never removed leaks a `longhorn-release-*`
// directory into the run scratch, which milestone QA found in two rounds
// (longhorn#174). Every fixture goes through one tracker: `settle` removes the
// fixtures this suite created and then proves no `longhorn-release-*` directory
// of unknown ownership is left. The baseline recorded at creation keeps another
// process's leftovers from failing this suite for something it did not create.
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const PREFIX = "longhorn-release-";

async function releaseScratchNames(): Promise<Array<string>> {
  const entries = await readdir(tmpdir()).catch(() => [] as Array<string>);
  return entries.filter((name) => name.startsWith(PREFIX));
}

export type ReleaseFixtureScratch = {
  create(prefix: string): Promise<string>;
  settle(): Promise<void>;
};

export async function releaseFixtureScratch(): Promise<ReleaseFixtureScratch> {
  const baseline = new Set(await releaseScratchNames());
  const created: Array<string> = [];
  return {
    async create(prefix) {
      const root = await mkdtemp(join(tmpdir(), prefix));
      created.push(root);
      return root;
    },
    async settle() {
      await Promise.all(
        created.splice(0).map((root) => rm(root, { recursive: true, force: true })),
      );
      const leaks = (await releaseScratchNames()).filter((name) => !baseline.has(name));
      if (leaks.length > 0) {
        throw new Error(
          `release-tooling fixtures left ${leaks.length} longhorn-release-* director${leaks.length === 1 ? "y" : "ies"} behind: ${leaks.join(", ")}`,
        );
      }
    },
  };
}
