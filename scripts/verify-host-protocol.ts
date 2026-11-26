import { isRecord } from "../packages/longhorn/src/core/record.ts";
import { parseJsonRecord, requireStrings } from "./proof-json.ts";
import { errorCode, requireValue } from "./proof-support.ts";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const defaultRepoRoot = resolve(import.meta.dir, "..");

export type HostProtocolSeams = {
  readonly commands: ReadonlyMap<string, string>;
  readonly events: ReadonlyMap<string, string>;
  readonly commandsRustOnly: ReadonlyMap<string, string>;
  readonly eventsRustOnly: ReadonlyMap<string, string>;
};

// Consumer-implemented seams: renderer packages define these names, and the
// app's own Tauri host implements them. They have no Rust implementation in
// this repo by contract.
const seamCommands = new Map([
  [
    "longhorn_native_content_connect",
    "consumer-implemented host; contract 017 native-content islands",
  ],
  [
    "longhorn_native_content_snapshot",
    "consumer-implemented host; contract 017 native-content islands",
  ],
  [
    "longhorn_native_content_update_desired",
    "consumer-implemented host; contract 017 native-content islands",
  ],
  [
    "longhorn_native_content_decide_size",
    "consumer-implemented host; contract 017 native-content islands",
  ],
  ["longhorn_surfaces_snapshot", "consumer-implemented host; surfaces window-host projection"],
  ["longhorn_surfaces_mutate", "consumer-implemented host; surfaces window-host projection"],
]);
const seamEvents = new Map([
  ["longhorn://native-content/changed", "consumer-emitted; contract 017 native-content islands"],
  ["longhorn://surfaces/changed", "consumer-emitted; surfaces window-host projection"],
  ["longhorn://agent-control/selection", "contract 022; MCP resource, not a Tauri event"],
]);

// Rust-only surface with no renderer wiring, consumer-filled by contract 010.
const seamCommandsRustOnly = new Map([
  [
    "longhorn_command_catalogue",
    "contract 010; renderer catalogue read surface is consumer-filled",
  ],
  ["longhorn_command_keymap", "contract 010; renderer keymap read surface is consumer-filled"],
  [
    "longhorn_command_keymap_preview",
    "contract 010; renderer keymap read surface is consumer-filled",
  ],
  [
    "longhorn_command_keymap_commit",
    "contract 010; renderer keymap read surface is consumer-filled",
  ],
  [
    "longhorn_command_keymap_reset",
    "contract 010; renderer keymap read surface is consumer-filled",
  ],
]);
const seamEventsRustOnly = new Map([
  ["longhorn://command/catalogue-changed", "contract 010; renderer listen is consumer-filled"],
  ["longhorn://command/keymap-changed", "contract 010; renderer listen is consumer-filled"],
]);

export const DEFAULT_SEAMS: HostProtocolSeams = {
  commands: seamCommands,
  events: seamEvents,
  commandsRustOnly: seamCommandsRustOnly,
  eventsRustOnly: seamEventsRustOnly,
};

function walk(directory: string, suffix: string, files: Array<string> = []): Array<string> {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    // Never descend into installed packages. This walks source for protocol
    // strings, and a dependency tree holds none that belong to this repository.
    if (entry.name === "node_modules") {
      continue;
    }
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      walk(path, suffix, files);
    } else if (path.endsWith(suffix)) {
      files.push(path);
    }
  }
  return files;
}

function collectSource(
  root: string,
  directories: Array<string>,
  matcher: RegExp,
  prefix: string,
  suffix: string,
  sourceSubdir?: string,
): Set<string> {
  const found = new Set<string>();
  for (const directory of directories) {
    const sourceRoot =
      sourceSubdir === undefined
        ? join(root, prefix, directory)
        : join(root, prefix, directory, sourceSubdir);
    for (const file of walk(sourceRoot, suffix)) {
      for (const match of readFileSync(file, "utf8").matchAll(matcher)) {
        found.add(requireValue(match[1], "match[1]"));
      }
    }
  }
  return found;
}

function optionalDirectoryEntries(directory: string): Array<string> {
  try {
    return readdirSync(directory);
  } catch (error) {
    if (errorCode(error) === "ENOENT") {
      return [];
    }
    throw error;
  }
}

export function verifyHostProtocol(
  repoRoot = defaultRepoRoot,
  seams: HostProtocolSeams = DEFAULT_SEAMS,
) {
  const tauriCrateNames = readdirSync(join(repoRoot, "crates")).filter((name) =>
    name.startsWith("longhorn-tauri-"),
  );
  const packageNames = readdirSync(join(repoRoot, "packages")).filter((name) => name !== "poodle");

  // Scan only `src/` on Rust crates so integration tests (`tests/`) and
  // examples never count as shipped Tauri commands or event emitters.
  const rustCommands = collectSource(
    repoRoot,
    tauriCrateNames,
    /#\[tauri::command[^\]]*\]\s*(?:pub\s+)?(?:async\s+)?fn\s+(longhorn_[a-z0-9_]+)/gs,
    "crates",
    ".rs",
    "src",
  );
  const rustEvents = collectSource(
    repoRoot,
    tauriCrateNames,
    /"(longhorn:\/\/[a-z0-9/-]+)"/g,
    "crates",
    ".rs",
    "src",
  );
  const tsCommands = collectSource(
    repoRoot,
    packageNames,
    /"(longhorn_[a-z0-9_]+)"/g,
    "packages",
    ".ts",
  );
  const tsEvents = collectSource(
    repoRoot,
    packageNames,
    /"(longhorn:\/\/[a-z0-9/-]+)"/g,
    "packages",
    ".ts",
  );

  const problems: Array<string> = [];

  for (const command of tsCommands) {
    if (!rustCommands.has(command) && !seams.commands.has(command)) {
      problems.push(`renderer invokes command with no Rust implementation and no seam: ${command}`);
    }
  }
  for (const command of rustCommands) {
    if (!tsCommands.has(command) && !seams.commandsRustOnly.has(command)) {
      problems.push(`Rust command with no renderer wiring and no documented seam: ${command}`);
    }
  }
  for (const event of tsEvents) {
    if (!rustEvents.has(event) && !seams.events.has(event)) {
      problems.push(`renderer listens for an event no Rust emitter and no seam provides: ${event}`);
    }
  }
  for (const event of rustEvents) {
    if (!tsEvents.has(event) && !seams.eventsRustOnly.has(event)) {
      problems.push(`Rust event with no renderer listener and no documented seam: ${event}`);
    }
  }

  for (const [command] of seams.commands) {
    if (seams.commandsRustOnly.has(command)) {
      problems.push(`command listed in both seamCommands and seamCommandsRustOnly: ${command}`);
    }
    if (rustCommands.has(command) || !tsCommands.has(command)) {
      problems.push(`stale seamCommands entry (not renderer-only): ${command}`);
    }
  }
  for (const [command] of seams.commandsRustOnly) {
    if (!rustCommands.has(command) || tsCommands.has(command)) {
      problems.push(`stale seamCommandsRustOnly entry (not Rust-only): ${command}`);
    }
  }
  for (const [event] of seams.events) {
    if (seams.eventsRustOnly.has(event)) {
      problems.push(`event listed in both seamEvents and seamEventsRustOnly: ${event}`);
    }
    if (rustEvents.has(event) || !tsEvents.has(event)) {
      problems.push(`stale seamEvents entry (not renderer-only): ${event}`);
    }
  }
  for (const [event] of seams.eventsRustOnly) {
    if (!rustEvents.has(event) || tsEvents.has(event)) {
      problems.push(`stale seamEventsRustOnly entry (not Rust-only): ${event}`);
    }
  }

  // Permission and capability examples must reference real commands and real
  // permission identifiers. Read all permissions across crates before checking
  // capabilities so cross-crate ordering never hides a declared identifier.
  const permissionIds = new Set<string>();
  const allowedCommands = new Set<string>();

  for (const directory of tauriCrateNames) {
    const permissionsRoot = join(repoRoot, "crates", directory, "examples", "permissions");
    for (const file of optionalDirectoryEntries(permissionsRoot)) {
      const relativeFile = join("crates", directory, "examples", "permissions", file);
      const parsed = Bun.TOML.parse(readFileSync(join(permissionsRoot, file), "utf8"));
      if (
        !isRecord(parsed) ||
        !Array.isArray(parsed.permission) ||
        parsed.permission.length === 0
      ) {
        problems.push(`${relativeFile} has no [[permission]] entries`);
        continue;
      }
      for (const entry of parsed.permission) {
        if (!isRecord(entry) || typeof entry.identifier !== "string" || entry.identifier === "") {
          problems.push(`${relativeFile} has no identifier`);
          continue;
        }
        permissionIds.add(entry.identifier);
        if (isRecord(entry.commands) && entry.commands.allow !== undefined) {
          for (const name of requireStrings(
            entry.commands.allow,
            `${relativeFile} commands.allow`,
          )) {
            allowedCommands.add(name);
          }
        }
      }
    }
  }

  for (const directory of tauriCrateNames) {
    const capabilitiesRoot = join(repoRoot, "crates", directory, "examples", "capabilities");
    for (const file of optionalDirectoryEntries(capabilitiesRoot)) {
      const relativeFile = join("crates", directory, "examples", "capabilities", file);
      const parsed = parseJsonRecord(readFileSync(join(capabilitiesRoot, file), "utf8"));
      if (parsed.permissions === undefined) {
        continue;
      }
      for (const name of requireStrings(parsed.permissions, `${relativeFile} permissions`)) {
        if (name.startsWith("core:")) {
          continue;
        }
        if (!permissionIds.has(name)) {
          problems.push(`${relativeFile} references undeclared permission ${name}`);
        }
      }
    }
  }

  for (const command of allowedCommands) {
    if (!rustCommands.has(command)) {
      problems.push(`permission allows command with no Rust implementation: ${command}`);
    }
  }

  if (problems.length > 0) {
    throw new Error(`host protocol drift: ${problems.length} problem(s):\n${problems.join("\n")}`);
  }

  return {
    schema: "longhorn.host-protocol-proof.v1",
    outcome: "pass",
    rustCommands: rustCommands.size,
    rendererCommands: tsCommands.size,
    seamCommands: seams.commands.size + seams.commandsRustOnly.size,
    rustEvents: rustEvents.size,
    rendererEvents: tsEvents.size,
    seamEvents: seams.events.size + seams.eventsRustOnly.size,
    permissions: permissionIds.size,
    allowedCommands: allowedCommands.size,
  };
}

if (import.meta.main) {
  try {
    console.log(JSON.stringify(verifyHostProtocol(), null, 2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
