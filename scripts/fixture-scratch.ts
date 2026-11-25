// Test-only scratch bookkeeping for the release-tooling suite.
//
// A `mkdtemp` fixture that is never removed leaks a `longhorn-release-*`
// directory into the run scratch, which milestone QA found in two rounds
// (longhorn#174). Every fixture goes through one tracker: `settle` removes the
// fixtures the suite created and then proves none is left behind.
//
// A directory named with one of the suite's own prefixes is judged absolutely:
// it may not survive the suite even if an earlier crashed run left it, which is
// how the recurring leak was found in milestone QA. A prefix is only known once
// the suite asks for it, so a fully independent process sharing the same scratch
// is the one case this cannot attribute. Any other `longhorn-release-*` name is
// judged against the baseline recorded when the tracker was created, so another
// suite's leftovers (for example `longhorn-release-runner-tools-*`, which
// removes its own fixtures in `afterEach`) cannot red this one.
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
  const ownPrefixes = new Set<string>();
  return {
    async create(prefix) {
      ownPrefixes.add(prefix);
      const root = await mkdtemp(join(tmpdir(), prefix));
      created.push(root);
      return root;
    },
    async settle() {
      await Promise.all(
        created.splice(0).map((root) => rm(root, { recursive: true, force: true })),
      );
      const remaining = await releaseScratchNames();
      const ownLeaks = remaining.filter((name) =>
        [...ownPrefixes].some((prefix) => name.startsWith(prefix)),
      );
      const otherLeaks = remaining.filter(
        (name) => !baseline.has(name) && !ownLeaks.includes(name),
      );
      const leaks = [...ownLeaks, ...otherLeaks];
      if (leaks.length > 0) {
        throw new Error(
          `release-tooling fixtures left ${leaks.length} longhorn-release-* director${leaks.length === 1 ? "y" : "ies"} behind: ${leaks.join(", ")}`,
        );
      }
    },
  };
}
