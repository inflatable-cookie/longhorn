import { afterEach, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { HostProtocolSeams } from "./verify-host-protocol.ts";
import { DEFAULT_SEAMS, verifyHostProtocol } from "./verify-host-protocol.ts";

const tempRoots: Array<string> = [];

afterEach(async () => {
  while (tempRoots.length > 0) {
    const root = tempRoots.pop();
    if (root) {
      await rm(root, { recursive: true, force: true });
    }
  }
});

const emptySeams: HostProtocolSeams = {
  commands: new Map(),
  events: new Map(),
  commandsRustOnly: new Map(),
  eventsRustOnly: new Map(),
};

async function fixtureRepo(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "longhorn-host-protocol-test-"));
  tempRoots.push(root);
  await mkdir(join(root, "crates", "longhorn-tauri-sample", "src"), { recursive: true });
  await mkdir(join(root, "packages", "sample", "src"), { recursive: true });
  return root;
}

describe("verify-host-protocol", () => {
  test("passes on the live repository with no overlapping or test-only agent-control seams", () => {
    for (const [event] of DEFAULT_SEAMS.events) {
      expect(DEFAULT_SEAMS.eventsRustOnly.has(event)).toBe(false);
    }
    const report = verifyHostProtocol();
    expect(report.outcome).toBe("pass");
    expect(report.rustEvents).toBe(14);
    expect(report.seamEvents).toBe(5);
  });

  test("ignores tests/ strings when counting Rust commands and emitters", async () => {
    const root = await fixtureRepo();
    await writeFile(
      join(root, "crates", "longhorn-tauri-sample", "src", "lib.rs"),
      `#[tauri::command]\npub fn longhorn_sample_ping() {}\npub const EV: &str = "longhorn://sample/changed";\n`,
    );
    await mkdir(join(root, "crates", "longhorn-tauri-sample", "tests"), { recursive: true });
    await writeFile(
      join(root, "crates", "longhorn-tauri-sample", "tests", "mount.rs"),
      `#[tauri::command]\nfn longhorn_sample_test_only() {}\nconst U: &str = "longhorn://agent-control/console";\n`,
    );
    await writeFile(
      join(root, "packages", "sample", "src", "index.ts"),
      `export const CMD = "longhorn_sample_ping";\nexport const EV = "longhorn://sample/changed";\n`,
    );

    const report = verifyHostProtocol(root, emptySeams);
    expect(report.rustCommands).toBe(1);
    expect(report.rustEvents).toBe(1);
  });

  test("still scans TypeScript tests/ files as renderer protocol references", async () => {
    const root = await fixtureRepo();
    await writeFile(
      join(root, "crates", "longhorn-tauri-sample", "src", "lib.rs"),
      `#[tauri::command]\npub fn longhorn_sample_ping() {}\npub const EV: &str = "longhorn://sample/changed";\n`,
    );
    await writeFile(
      join(root, "packages", "sample", "src", "index.ts"),
      `export const EV = "longhorn://sample/changed";\n`,
    );
    await mkdir(join(root, "packages", "sample", "tests"), { recursive: true });
    await writeFile(
      join(root, "packages", "sample", "tests", "client.test.ts"),
      `export const CMD = "longhorn_sample_ping";\nexport const PHANTOM = "longhorn_sample_missing";\n`,
    );

    expect(() => verifyHostProtocol(root, emptySeams)).toThrow(
      /renderer invokes command with no Rust implementation and no seam: longhorn_sample_missing/,
    );
  });

  test("does not swallow malformed permissions TOML or capabilities JSON", async () => {
    const root = await fixtureRepo();
    await mkdir(join(root, "crates", "longhorn-tauri-sample", "examples", "permissions"), {
      recursive: true,
    });
    await writeFile(
      join(root, "crates", "longhorn-tauri-sample", "examples", "permissions", "bad.toml"),
      "[[permission]\nidentifier = ",
    );
    expect(() => verifyHostProtocol(root, emptySeams)).toThrow();

    await rm(join(root, "crates", "longhorn-tauri-sample", "examples", "permissions", "bad.toml"));
    await mkdir(join(root, "crates", "longhorn-tauri-sample", "examples", "capabilities"), {
      recursive: true,
    });
    await writeFile(
      join(root, "crates", "longhorn-tauri-sample", "examples", "capabilities", "bad.json"),
      "{not valid json",
    );
    expect(() => verifyHostProtocol(root, emptySeams)).toThrow();
  });

  test("rejects overlapping or stale seam declarations", async () => {
    const root = await fixtureRepo();
    await writeFile(
      join(root, "packages", "sample", "src", "index.ts"),
      `export const EV = "longhorn://agent-control/selection";\n`,
    );
    const overlappingSeams: HostProtocolSeams = {
      commands: new Map(),
      events: new Map([["longhorn://agent-control/selection", "client"]]),
      commandsRustOnly: new Map(),
      eventsRustOnly: new Map([["longhorn://agent-control/selection", "rust"]]),
    };

    expect(() => verifyHostProtocol(root, overlappingSeams)).toThrow(
      /event listed in both seamEvents and seamEventsRustOnly/,
    );
  });
});
