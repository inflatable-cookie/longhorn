#!/usr/bin/env bun
// Enumerate live agent-control discovery files (contract 022).
//
// Prints the instance URL and a ready-to-paste `claude mcp add` line.
// The bearer token appears only on that paste line, never in diagnostics.
//
// Usage:
//   bun find-instance.ts [--app-id <id>] [--discovery-dir <path>]

import { readdir, readFile, unlink } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";

export const DISCOVERY_SCHEMA_VERSION = 1;

// The highest TCP port. A discovery file naming a port outside 1-65535 is unreadable.
const MAX_PORT = 65_535;

export type DiscoveryFile = {
  schemaVersion: number;
  appId: string;
  pid: number;
  port: number;
  token: string;
};

export type LiveInstance = {
  path: string;
  file: DiscoveryFile;
};

// The options `find-instance` takes from its arguments.
export type FinderOptions = {
  appId?: string;
  discoveryDir?: string;
};

export type ScanResult = {
  live: Array<LiveInstance>;
  /** Dead-pid files successfully unlinked. */
  removed: number;
  /** Dead-pid files still present because unlink failed. */
  unlinkFailed: number;
  unreadable: number;
};

export function defaultDiscoveryDir(): string {
  if (process.env.LONGHORN_AGENT_CONTROL_DISCOVERY_DIR) {
    return process.env.LONGHORN_AGENT_CONTROL_DISCOVERY_DIR;
  }
  if (process.platform === "darwin") {
    return join(homedir(), "Library", "Application Support", "longhorn", "state", "agent-control");
  }
  if (process.platform === "win32") {
    const local = process.env.LOCALAPPDATA;
    if (!local) {
      throw new Error("LOCALAPPDATA is unset; cannot resolve the discovery directory");
    }
    return join(local, "longhorn", "state", "agent-control");
  }
  const stateHome = process.env.XDG_STATE_HOME ?? join(homedir(), ".local", "state");
  return join(stateHome, "longhorn", "agent-control");
}

// `kill(pid, 0)` signals nothing; it only asks whether the pid exists. ESRCH is
// the one answer that proves a process gone. EPERM means it exists under another
// user, so it is alive: unlinking its discovery file would hide a live instance.
// Any other error also counts as alive, because only a proven-dead pid may lose
// its file.
export function pidAlive(
  pid: number,
  kill: (target: number, signal: number) => boolean = (target, signal) =>
    process.kill(target, signal),
): boolean {
  try {
    kill(pid, 0);
    return true;
  } catch (error) {
    return errorCode(error) !== "ESRCH";
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function errorCode(error: unknown): string | undefined {
  if (isRecord(error) && typeof error["code"] === "string") {
    return error["code"];
  }
  return undefined;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

// A record is trusted only when its own fields agree with its file name, which
// carries the app id and pid, and its port and token are usable as written.
function parseRecord(name: string, text: string): DiscoveryFile | null {
  if (!name.endsWith(".json")) {
    return null;
  }
  const stem = name.slice(0, -".json".length);
  const split = stem.lastIndexOf("-");
  if (split <= 0) {
    return null;
  }
  const appId = stem.slice(0, split);
  const pid = Number.parseInt(stem.slice(split + 1), 10);
  if (!Number.isInteger(pid) || pid < 0) {
    return null;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) {
    return null;
  }
  const { schemaVersion, appId: recordedAppId, pid: recordedPid, port, token } = parsed;
  if (
    schemaVersion !== DISCOVERY_SCHEMA_VERSION ||
    recordedAppId !== appId ||
    recordedPid !== pid ||
    typeof port !== "number" ||
    !Number.isInteger(port) ||
    port < 1 ||
    port > MAX_PORT ||
    typeof token !== "string" ||
    token.length === 0
  ) {
    return null;
  }
  return { schemaVersion: DISCOVERY_SCHEMA_VERSION, appId, pid, port, token };
}

export async function scanDiscovery(
  dir: string,
  appId: string | undefined,
  alive: (pid: number) => boolean = pidAlive,
  removeFile: (path: string) => Promise<void> = unlink,
): Promise<ScanResult> {
  let names: Array<string>;
  try {
    names = await readdir(dir);
  } catch (error) {
    if (errorCode(error) === "ENOENT") {
      return { live: [], removed: 0, unlinkFailed: 0, unreadable: 0 };
    }
    throw error;
  }

  const result: ScanResult = { live: [], removed: 0, unlinkFailed: 0, unreadable: 0 };
  for (const name of names) {
    const path = join(dir, name);
    let text: string;
    try {
      text = await readFile(path, "utf8");
    } catch {
      result.unreadable += 1;
      continue;
    }
    const file = parseRecord(name, text);
    if (!file) {
      result.unreadable += 1;
      continue;
    }
    if (!alive(file.pid)) {
      try {
        await removeFile(path);
        result.removed += 1;
      } catch {
        // Best-effort: still skip a dead-pid file we cannot unlink.
        result.unlinkFailed += 1;
      }
      continue;
    }
    if (appId && file.appId !== appId) {
      continue;
    }
    result.live.push({ path, file });
  }
  result.live.sort((left, right) => {
    const byApp = left.file.appId.localeCompare(right.file.appId);
    return byApp !== 0 ? byApp : left.file.pid - right.file.pid;
  });
  return result;
}

export function instanceUrl(file: DiscoveryFile): string {
  return `http://127.0.0.1:${file.port}/mcp`;
}

// Claude Code server names admit only letters, numbers, hyphens, and
// underscores; reverse-DNS app ids carry dots, so every unsafe symbol
// becomes a hyphen (Soundcheck adoption finding, 2026-08-19).
export function mcpServerName(file: DiscoveryFile): string {
  const appId = file.appId.replaceAll(/[^A-Za-z0-9_-]+/g, "-");
  return `longhorn-${appId}-${file.pid}`;
}

export function mcpAddLine(file: DiscoveryFile): string {
  const url = instanceUrl(file);
  return `claude mcp add --transport http ${mcpServerName(file)} ${url} --header "Authorization: Bearer ${file.token}"`;
}

export function formatStdout(live: Array<LiveInstance>): string {
  return live.map(({ file }) => `url: ${instanceUrl(file)}\n${mcpAddLine(file)}`).join("\n\n");
}

export function formatDiagnostics(scan: ScanResult, appId: string | undefined): string {
  const lines: Array<string> = [];
  if (scan.live.length === 0) {
    lines.push(
      appId
        ? `no live agent-control instance for app id ${appId}`
        : "no live agent-control instance",
    );
  } else {
    lines.push(`found ${scan.live.length} live instance${scan.live.length === 1 ? "" : "s"}`);
    for (const { file } of scan.live) {
      lines.push(`  appId=${file.appId} pid=${file.pid} port=${file.port}`);
    }
  }
  if (scan.removed > 0) {
    lines.push(`removed ${scan.removed} stale file${scan.removed === 1 ? "" : "s"} (dead pid)`);
  }
  if (scan.unlinkFailed > 0) {
    lines.push(
      `skipped ${scan.unlinkFailed} stale file${scan.unlinkFailed === 1 ? "" : "s"} (dead pid; unlink failed)`,
    );
  }
  if (scan.unreadable > 0) {
    lines.push(`skipped ${scan.unreadable} unreadable file${scan.unreadable === 1 ? "" : "s"}`);
  }
  return lines.join("\n");
}

export function parseArgs(argv: Array<string>): FinderOptions {
  const out: FinderOptions = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index] ?? "";
    const next = argv[index + 1];
    if (arg === "--app-id" && next) {
      out.appId = next;
      index += 1;
      continue;
    }
    if (arg === "--discovery-dir" && next) {
      out.discoveryDir = next;
      index += 1;
      continue;
    }
    if (arg === "--help" || arg === "-h") {
      throw new Error("usage: find-instance.ts [--app-id <id>] [--discovery-dir <path>]");
    }
    throw new Error(`unknown argument ${arg}`);
  }
  return out;
}

export async function main(argv = process.argv.slice(2)): Promise<number> {
  let options: FinderOptions;
  try {
    options = parseArgs(argv);
  } catch (error) {
    console.error(errorMessage(error));
    return 2;
  }
  const dir = options.discoveryDir ?? defaultDiscoveryDir();
  const scan = await scanDiscovery(dir, options.appId);
  console.error(formatDiagnostics(scan, options.appId));
  if (scan.live.length === 0) {
    return 1;
  }
  console.log(formatStdout(scan.live));
  return 0;
}

if (import.meta.main) {
  process.exit(await main());
}
