import { afterEach, describe, expect, test } from "bun:test";
import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { pidAlive, scanDiscovery } from "../skills/agent-control/scripts/find-instance.ts";

const created: Array<string> = [];

afterEach(async () => {
  for (const dir of created.splice(0)) {
    await rm(dir, { recursive: true, force: true });
  }
});

// A `kill` stand-in that fails with the given errno code, as the OS does.
function killFailingWith(code: string): (target: number, signal: number) => boolean {
  return () => {
    throw Object.assign(new Error(`kill ${code}`), { code });
  };
}

// The pid probes a scan sees on a host where the process exists under another
// user (EPERM) or is gone (ESRCH).
const pidAliveUnderEperm = (pid: number) => pidAlive(pid, killFailingWith("EPERM"));
const pidAliveUnderEsrch = (pid: number) => pidAlive(pid, killFailingWith("ESRCH"));

async function discoveryDir(records: Record<string, unknown>): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), "agent-control-finder-"));
  created.push(dir);
  for (const [name, record] of Object.entries(records)) {
    await writeFile(join(dir, name), JSON.stringify(record));
  }
  return dir;
}

function record(overrides: Record<string, unknown> = {}) {
  return {
    schemaVersion: 1,
    appId: "com.example.app",
    pid: 4242,
    port: 8080,
    token: "token",
    ...overrides,
  };
}

describe("pidAlive", () => {
  test("ESRCH is the one answer that proves a process gone", () => {
    expect(pidAlive(4242, killFailingWith("ESRCH"))).toBe(false);
  });

  test("EPERM means the process exists under another user, so it is alive", () => {
    expect(pidAlive(1, killFailingWith("EPERM"))).toBe(true);
  });

  test("any other error is treated as alive, and a successful probe is alive", () => {
    expect(pidAlive(4242, killFailingWith("EINVAL"))).toBe(true);
    expect(pidAlive(4242, () => true)).toBe(true);
  });
});

describe("scanDiscovery", () => {
  test("a live instance behind EPERM keeps its discovery file", async () => {
    const dir = await discoveryDir({ "com.example.app-4242.json": record() });
    const scan = await scanDiscovery(dir, undefined, pidAliveUnderEperm);
    expect(scan.live.map(({ file }) => file.pid)).toEqual([4242]);
    expect(scan.removed).toBe(0);
    expect(await readdir(dir)).toEqual(["com.example.app-4242.json"]);
  });

  test("a dead pid's discovery file is removed", async () => {
    const dir = await discoveryDir({ "com.example.app-4242.json": record() });
    const scan = await scanDiscovery(dir, undefined, pidAliveUnderEsrch);
    expect(scan.live).toEqual([]);
    expect(scan.removed).toBe(1);
    expect(await readdir(dir)).toEqual([]);
  });

  test("a port outside 1-65_535, or not an integer, makes the record unreadable", async () => {
    const dir = await discoveryDir({
      "com.example.app-1.json": record({ pid: 1, port: 0 }),
      "com.example.app-2.json": record({ pid: 2, port: 65_536 }),
      "com.example.app-3.json": record({ pid: 3, port: 1.5 }),
      "com.example.app-4.json": record({ pid: 4, port: "8080" }),
      "com.example.app-5.json": record({ pid: 5, port: 65_535 }),
    });
    const scan = await scanDiscovery(dir, undefined, () => true);
    expect(scan.live.map(({ file }) => file.port)).toEqual([65_535]);
    expect(scan.unreadable).toBe(4);
  });

  test("a record whose fields disagree with its file name is unreadable", async () => {
    const dir = await discoveryDir({ "com.example.app-4242.json": record({ pid: 7 }) });
    const scan = await scanDiscovery(dir, undefined, () => true);
    expect(scan.live).toEqual([]);
    expect(scan.unreadable).toBe(1);
  });
});
