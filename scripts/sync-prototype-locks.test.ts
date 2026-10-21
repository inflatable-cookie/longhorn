import { describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { addMissingLonghornPathPackages, parseLock } from "./sync-prototype-locks.ts";

const repoRoot = resolve(import.meta.dir, "..");

describe("prototype lock additions", () => {
  test("restores a missing Longhorn path package and its dependency edges", async () => {
    const fixture = await mkdtemp(join(tmpdir(), "longhorn-prototype-lock-add-"));
    try {
      const prototypeDir = join(fixture, "prototype");
      await mkdir(prototypeDir, { recursive: true });
      const workspaceLock = await readFile(join(repoRoot, "Cargo.lock"), "utf8");
      const prototypeLock = await readFile(join(repoRoot, "prototypes/gpui-composition/Cargo.lock"), "utf8");
      const workspacePackage = parseLock(workspaceLock).packages.find((pkg) => pkg.name === "longhorn-storage-layout");
      if (!workspacePackage) throw new Error("workspace lock has no longhorn-storage-layout package");
      const fullCopy = parseLock(prototypeLock).packages.some((pkg) => pkg.name === workspacePackage.name)
        ? prototypeLock
        : appendPackage(prototypeLock, workspacePackage.body);
      const copiedLock = join(prototypeDir, "Cargo.lock");
      await writeFile(copiedLock, removePackage(fullCopy, "longhorn-storage-layout"));

      const missingPackageCopy = await readFile(copiedLock, "utf8");
      const result = addMissingLonghornPathPackages(missingPackageCopy, workspaceLock);
      const added = parseLock(result.text).packages.find((pkg) => pkg.name === "longhorn-storage-layout");

      expect(result.added).toEqual(["longhorn-storage-layout"]);
      expect(added?.body).toEqual(workspacePackage.body);
    } finally {
      await rm(fixture, { recursive: true, force: true });
    }
  });

  test("refuses a missing third-party package instead of adding it", async () => {
    const fixture = await mkdtemp(join(tmpdir(), "longhorn-prototype-lock-third-party-"));
    try {
      const prototypeDir = join(fixture, "prototype");
      await mkdir(prototypeDir, { recursive: true });
      const workspaceLock = await readFile(join(repoRoot, "Cargo.lock"), "utf8");
      const prototypeLock = await readFile(join(repoRoot, "prototypes/gpui-composition/Cargo.lock"), "utf8");
      const copiedLock = join(prototypeDir, "Cargo.lock");
      await writeFile(copiedLock, removePackage(prototypeLock, "serde"));

      const missingThirdPartyCopy = await readFile(copiedLock, "utf8");
      expect(() => addMissingLonghornPathPackages(missingThirdPartyCopy, workspaceLock)).toThrow(
        /missing third-party package serde/,
      );
    } finally {
      await rm(fixture, { recursive: true, force: true });
    }
  });
});

function removePackage(lockText: string, packageName: string): string {
  const blocks = lockText.split(/^\[\[package\]\]\n/m);
  const header = blocks.shift() ?? "";
  const kept = blocks.filter((block) => /^name = "([^"]+)"/m.exec(block)?.[1] !== packageName);
  return [header, ...kept.map((block) => `[[package]]\n${block}`)].join("");
}

function appendPackage(lockText: string, body: string): string {
  return `${lockText.endsWith("\n") ? lockText : `${lockText}\n`}[[package]]\n${body}`;
}
