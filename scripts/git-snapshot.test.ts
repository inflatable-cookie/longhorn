import { afterEach, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const snapshotScript = resolve(import.meta.dir, "git-snapshot.sh");
const tempRoots: Array<string> = [];

afterEach(async () => {
  while (tempRoots.length > 0) {
    const root = tempRoots.pop();
    if (root) {
      await rm(root, { recursive: true, force: true });
    }
  }
});

function git(cwd: string, args: ReadonlyArray<string>): string {
  const result = spawnSync("git", [...args], { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(" ")} failed:\n${result.stderr}`);
  }
  return result.stdout.trim();
}

async function createRepo(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "longhorn-git-snapshot-test-"));
  tempRoots.push(root);
  git(root, ["init", "-q"]);
  await writeFile(join(root, "tracked.txt"), "initial\n");
  await writeFile(join(root, "deleted.txt"), "remove me\n");
  git(root, ["add", "-A"]);
  git(root, [
    "-c",
    "user.name=Test",
    "-c",
    "user.email=test@invalid",
    "-c",
    "commit.gpgsign=false",
    "commit",
    "-q",
    "-m",
    "Initial commit",
  ]);
  return root;
}

function runSnapshot(repo: string, destination: string, message?: string) {
  const args = [snapshotScript, destination, ...(message ? [message] : [])];
  const result = spawnSync("bash", args, { cwd: repo, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`git-snapshot.sh failed (${result.status}):\n${result.stderr}`);
  }
  const [kind, root, commit] = result.stdout.trim().split("\n");
  return { kind, root, commit };
}

describe("git-snapshot.sh", () => {
  test("resolves a clean working tree directly at HEAD without creating a snapshot", async () => {
    const repo = await createRepo();
    const destination = join(repo, ".unused-snapshot");
    const head = git(repo, ["rev-parse", "HEAD"]);

    const snapshot = runSnapshot(repo, destination);
    expect(snapshot.kind).toBe("commit");
    expect(snapshot.commit).toBe(head);
    expect(snapshot.root).toBeTruthy();
    expect(git(repo, ["rev-parse", "HEAD"])).toBe(head);
  });

  test("snapshots dirty modified, untracked, deleted and symlinked files deterministically", async () => {
    const repo = await createRepo();
    const destinationParent = await mkdtemp(join(tmpdir(), "longhorn-git-snapshot-dest-"));
    tempRoots.push(destinationParent);
    const destination = join(destinationParent, "snapshot");

    await writeFile(join(repo, "tracked.txt"), "modified\n");
    await mkdir(join(repo, "nested"));
    await writeFile(join(repo, "nested", "untracked.txt"), "new\n");
    await rm(join(repo, "deleted.txt"));
    await symlink("tracked.txt", join(repo, "link.txt"));

    const snapshot = runSnapshot(repo, destination, "Dirty test snapshot");
    expect(snapshot.kind).toBe("snapshot");
    expect(snapshot.root).toBe(await readlinkSafe(destination));
    expect(snapshot.commit).toMatch(/^[0-9a-f]{40}$/);

    expect(await readFile(join(destination, "tracked.txt"), "utf8")).toBe("modified\n");
    expect(await readFile(join(destination, "nested", "untracked.txt"), "utf8")).toBe("new\n");
    expect(await readFile(join(destination, "link.txt"), "utf8")).toBe("modified\n");
    expect(git(destination, ["status", "--porcelain"])).toBe("");
    expect(git(destination, ["show", "-s", "--format=%aI|%cI|%s", "HEAD"])).toBe(
      "2000-01-01T00:00:00Z|2000-01-01T00:00:00Z|Dirty test snapshot",
    );
    expect(git(destination, ["ls-files"])).not.toContain("deleted.txt");
  });
});

async function readlinkSafe(path: string): Promise<string> {
  const { realpath } = await import("node:fs/promises");
  return realpath(path);
}
