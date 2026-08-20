#!/usr/bin/env bun
// Enumerate live agent-control discovery files (contract 022).
//
// Prints the instance URL and a ready-to-paste `claude mcp add` line.
// The bearer token appears only on that paste line, never in diagnostics.
//
// Usage:
//   bun find-instance.ts [--app-id <id>] [--discovery-dir <path>]

import { readdir, readFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";

export const DISCOVERY_SCHEMA_VERSION = 1;

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

export type ScanResult = {
  live: LiveInstance[];
  stale: number;
  unreadable: number;
};

export function defaultDiscoveryDir(): string {
  if (process.env.LONGHORN_AGENT_CONTROL_DISCOVERY_DIR) {
    return process.env.LONGHORN_AGENT_CONTROL_DISCOVERY_DIR;
  }
  if (process.platform === "darwin") {
    return join(
      homedir(),
      "Library",
      "Application Support",
      "longhorn",
      "state",
      "agent-control",
    );
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

export function pidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function parseRecord(name: string, text: string): DiscoveryFile | null {
  if (!name.endsWith(".json")) return null;
  const stem = name.slice(0, -".json".length);
  const split = stem.lastIndexOf("-");
  if (split <= 0) return null;
  const appId = stem.slice(0, split);
  const pid = Number.parseInt(stem.slice(split + 1), 10);
  if (!Number.isInteger(pid) || pid < 0) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (!parsed || typeof parsed !== "object") return null;
  const record = parsed as Partial<DiscoveryFile>;
  if (
    record.schemaVersion !== DISCOVERY_SCHEMA_VERSION ||
    record.appId !== appId ||
    record.pid !== pid ||
    typeof record.port !== "number" ||
    typeof record.token !== "string" ||
    record.token.length === 0
  ) {
    return null;
  }
  return record as DiscoveryFile;
}

export async function scanDiscovery(
  dir: string,
  appId: string | undefined,
  alive: (pid: number) => boolean = pidAlive,
): Promise<ScanResult> {
  let names: string[];
  try {
    names = await readdir(dir);
  } catch (error) {
    const code = (error as { code?: string }).code;
    if (code === "ENOENT") {
      return { live: [], stale: 0, unreadable: 0 };
    }
    throw error;
  }

  const result: ScanResult = { live: [], stale: 0, unreadable: 0 };
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
    if (appId && file.appId !== appId) continue;
    if (!alive(file.pid)) {
      result.stale += 1;
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
  const appId = file.appId.replace(/[^A-Za-z0-9_-]+/g, "-");
  return `longhorn-${appId}-${file.pid}`;
}

export function mcpAddLine(file: DiscoveryFile): string {
  const url = instanceUrl(file);
  return `claude mcp add --transport http ${mcpServerName(file)} ${url} --header "Authorization: Bearer ${file.token}"`;
}

export function formatStdout(live: LiveInstance[]): string {
  return live
    .map(({ file }) => `url: ${instanceUrl(file)}\n${mcpAddLine(file)}`)
    .join("\n\n");
}

export function formatDiagnostics(scan: ScanResult, appId: string | undefined): string {
  const lines: string[] = [];
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
  if (scan.stale > 0) {
    lines.push(`skipped ${scan.stale} stale file${scan.stale === 1 ? "" : "s"} (dead pid)`);
  }
  if (scan.unreadable > 0) {
    lines.push(`skipped ${scan.unreadable} unreadable file${scan.unreadable === 1 ? "" : "s"}`);
  }
  return lines.join("\n");
}

export function parseArgs(argv: string[]): { appId?: string; discoveryDir?: string } {
  const out: { appId?: string; discoveryDir?: string } = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
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
      throw new Error(
        "usage: find-instance.ts [--app-id <id>] [--discovery-dir <path>]",
      );
    }
    throw new Error(`unknown argument ${arg}`);
  }
  return out;
}

export async function main(argv = process.argv.slice(2)): Promise<number> {
  let options: { appId?: string; discoveryDir?: string };
  try {
    options = parseArgs(argv);
  } catch (error) {
    console.error((error as Error).message);
    return 2;
  }
  const dir = options.discoveryDir ?? defaultDiscoveryDir();
  const scan = await scanDiscovery(dir, options.appId);
  console.error(formatDiagnostics(scan, options.appId));
  if (scan.live.length === 0) return 1;
  console.log(formatStdout(scan.live));
  return 0;
}

if (import.meta.main) {
  process.exit(await main());
}
