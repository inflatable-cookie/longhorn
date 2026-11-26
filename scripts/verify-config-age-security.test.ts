import { afterEach, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  assertLockedRefusal,
  removeStagedDirectories,
  runWithStagedCleanup,
  VULNERABLE_AGE_VERSION,
} from "./verify-config-age-security.ts";

const tempRoots: Array<string> = [];

afterEach(async () => {
  while (tempRoots.length > 0) {
    const root = tempRoots.pop();
    if (root) {
      await rm(root, { recursive: true, force: true });
    }
  }
});

describe("verify-config-age-security locked refusal check", () => {
  test("accepts Cargo's lockfile refusal messages", () => {
    expect(() =>
      assertLockedRefusal({
        ok: false,
        exitCode: 101,
        output:
          "error: cannot update the lock file /scratch/Cargo.lock because --locked was passed to prevent this",
      }),
    ).not.toThrow();

    expect(() =>
      assertLockedRefusal({
        ok: false,
        exitCode: 101,
        output:
          "error: the lock file /scratch/Cargo.lock needs to be updated but --locked was passed to prevent this",
      }),
    ).not.toThrow();
  });

  test("rejects a build that succeeded under --locked", () => {
    expect(() =>
      assertLockedRefusal({ ok: true, exitCode: 0, output: "" }, VULNERABLE_AGE_VERSION),
    ).toThrow(/still built --locked/);
  });

  test("rejects unrelated git, network, Oxide or spawn failures", () => {
    for (const failure of [
      { ok: false, exitCode: 128, output: "fatal: repository 'file:///missing' does not exist" },
      { ok: false, exitCode: 101, output: "error: failed to download from https://crates.io" },
      { ok: false, exitCode: 1, output: "oxide: failed to bind lease socket: path too long" },
      { ok: false, exitCode: 1, output: "" },
      { ok: false, exitCode: null, output: "", error: "ENOENT" },
    ]) {
      expect(() => assertLockedRefusal(failure)).toThrow(/unexpected reason|failed to launch/);
    }
  });
});

describe("verify-config-age-security staged cleanup and receipt ordering", () => {
  test("derives stagedTemporaryRemoved from actual post-cleanup directory state", async () => {
    const dirA = await mkdtemp(join(tmpdir(), "longhorn-config-age-test-a-"));
    const dirB = await mkdtemp(join(tmpdir(), "longhorn-config-age-test-b-"));
    tempRoots.push(dirA, dirB);

    expect(await removeStagedDirectories([dirA], async () => {})).toBe(false);
    expect(existsSync(dirA)).toBe(true);

    expect(await removeStagedDirectories([dirA, undefined, dirB])).toBe(true);
    expect(existsSync(dirA)).toBe(false);
    expect(existsSync(dirB)).toBe(false);
  });

  test("removes staged paths before returning the receipt and cleans up on failure", async () => {
    const stageDir = await mkdtemp(join(tmpdir(), "longhorn-config-age-test-order-"));
    tempRoots.push(stageDir);
    let existedDuringExecute = false;
    let disposed = false;

    const receipt = await runWithStagedCleanup(
      async () => {
        existedDuringExecute = existsSync(stageDir);
        return {
          schema: "longhorn.config-age-security-receipt.v1",
          outcome: "pass",
          sources: { repository: "/repo", snapshot: "commit", commit: "abc" },
          resolution: {
            repairedAge: "0.11.5",
            repairedAgeCorePrefix: "0.11.",
            vulnerableChainAbsent: ["crypto-bigint"],
            gitSourceAtExactCommit: true,
            unlockedLockfileSha256: "deadbeef",
          },
          staleLockProbe: {
            pinnedAge: VULNERABLE_AGE_VERSION,
            lockedBuildRefused: true,
            relockResolvedClean: true,
          },
          consumerRustVersion: "1.97.1",
        };
      },
      () => [stageDir],
      async () => {
        disposed = true;
      },
    );

    expect(existedDuringExecute).toBe(true);
    expect(existsSync(stageDir)).toBe(false);
    expect(disposed).toBe(true);
    expect(receipt.stagedTemporaryRemoved).toBe(true);

    const failingStageDir = await mkdtemp(join(tmpdir(), "longhorn-config-age-test-fail-"));
    tempRoots.push(failingStageDir);
    await expect(
      runWithStagedCleanup(
        async () => {
          throw new Error("boom");
        },
        () => [failingStageDir],
      ),
    ).rejects.toThrow("boom");
    expect(existsSync(failingStageDir)).toBe(false);
  });
});
