import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

// Consumer-implemented seams: renderer packages define these names, and the
// app's own Tauri host implements them. They have no Rust implementation in
// this repo by contract.
const seamCommands = new Map([
  ["longhorn_native_content_connect", "consumer-implemented host; contract 017 native-content islands"],
  ["longhorn_native_content_snapshot", "consumer-implemented host; contract 017 native-content islands"],
  ["longhorn_native_content_update_desired", "consumer-implemented host; contract 017 native-content islands"],
  ["longhorn_native_content_decide_size", "consumer-implemented host; contract 017 native-content islands"],
  ["longhorn_surfaces_snapshot", "consumer-implemented host; surfaces window-host projection"],
  ["longhorn_surfaces_mutate", "consumer-implemented host; surfaces window-host projection"],
]);
const seamEvents = new Map([
  ["longhorn://native-content/changed", "consumer-emitted; contract 017 native-content islands"],
  ["longhorn://surfaces/changed", "consumer-emitted; surfaces window-host projection"],
]);

// Rust-only surface with no renderer wiring, consumer-filled by contract 010.
const seamCommandsRustOnly = new Map([
  ["longhorn_command_catalogue", "contract 010; renderer catalogue read surface is consumer-filled"],
  ["longhorn_command_keymap", "contract 010; renderer keymap read surface is consumer-filled"],
  ["longhorn_command_keymap_preview", "contract 010; renderer keymap read surface is consumer-filled"],
  ["longhorn_command_keymap_commit", "contract 010; renderer keymap read surface is consumer-filled"],
  ["longhorn_command_keymap_reset", "contract 010; renderer keymap read surface is consumer-filled"],
]);
const seamEventsRustOnly = new Map([
  ["longhorn://command/catalogue-changed", "contract 010; renderer listen is consumer-filled"],
  ["longhorn://command/keymap-changed", "contract 010; renderer listen is consumer-filled"],
]);

function walk(directory: string, suffix: string, files: string[] = []): string[] {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    // Never descend into installed packages. This walks source for protocol
    // strings, and a dependency tree holds none that belong to this repository.
    // It also cannot be walked safely: `effigy deps link bun ../poodle` leaves
    // dangling symlinks in the linked package's own node_modules, and the
    // `statSync` this used to call follows symlinks and throws ENOENT on them.
    if (entry.name === "node_modules") continue;
    const path = join(directory, entry.name);
    // Dirent classification does not follow symlinks, so a broken link is a
    // file that does not end in the suffix rather than an exception.
    if (entry.isDirectory()) {
      walk(path, suffix, files);
    } else if (path.endsWith(suffix)) {
      files.push(path);
    }
  }
  return files;
}

function collect(
  root: string,
  directories: string[],
  matcher: RegExp,
  prefix: string,
  suffix: string,
): Set<string> {
  const found = new Set<string>();
  for (const directory of directories) {
    for (const file of walk(join(root, prefix, directory), suffix)) {
      for (const match of readFileSync(file, "utf8").matchAll(matcher)) {
        found.add(match[1]!);
      }
    }
  }
  return found;
}

const tauriCrateNames = readdirSync(join(repoRoot, "crates")).filter((name) =>
  name.startsWith("longhorn-tauri-"),
);
const packageNames = readdirSync(join(repoRoot, "packages")).filter((name) => name !== "poodle");

const rustCommands = collect(repoRoot, tauriCrateNames, /#\[tauri::command[^\]]*\]\s*(?:pub\s+)?(?:async\s+)?fn\s+(longhorn_[a-z0-9_]+)/gs, "crates", ".rs");
const rustEvents = collect(repoRoot, tauriCrateNames, /"(longhorn:\/\/[a-z0-9/-]+)"/g, "crates", ".rs");
const tsCommands = collect(repoRoot, packageNames, /"(longhorn_[a-z0-9_]+)"/g, "packages", ".ts");
const tsEvents = collect(repoRoot, packageNames, /"(longhorn:\/\/[a-z0-9/-]+)"/g, "packages", ".ts");

const problems: string[] = [];

for (const command of tsCommands) {
  if (!rustCommands.has(command) && !seamCommands.has(command)) {
    problems.push(`renderer invokes command with no Rust implementation and no seam: ${command}`);
  }
}
for (const command of rustCommands) {
  if (!tsCommands.has(command) && !seamCommandsRustOnly.has(command)) {
    problems.push(`Rust command with no renderer wiring and no documented seam: ${command}`);
  }
}
for (const event of tsEvents) {
  if (!rustEvents.has(event) && !seamEvents.has(event)) {
    problems.push(`renderer listens for an event no Rust emitter and no seam provides: ${event}`);
  }
}
for (const event of rustEvents) {
  if (!tsEvents.has(event) && !seamEventsRustOnly.has(event)) {
    problems.push(`Rust event with no renderer listener and no documented seam: ${event}`);
  }
}

// Permission and capability examples must reference real commands and real
// permission identifiers.
const permissionIds = new Set<string>();
const allowedCommands = new Set<string>();
for (const directory of readdirSync(join(repoRoot, "crates")).filter((name) =>
  name.startsWith("longhorn-tauri-"),
)) {
  const permissionsRoot = join(repoRoot, "crates", directory, "examples", "permissions");
  const capabilitiesRoot = join(repoRoot, "crates", directory, "examples", "capabilities");
  try {
    for (const file of readdirSync(permissionsRoot)) {
      const content = readFileSync(join(permissionsRoot, file), "utf8");
      const identifier = content.match(/identifier\s*=\s*"([^"]+)"/)?.[1];
      if (identifier === undefined) {
        problems.push(`${join("crates", directory, "examples", "permissions", file)} has no identifier`);
        continue;
      }
      permissionIds.add(identifier);
      for (const command of content.matchAll(/commands\.allow\s*=\s*\[([^\]]*)\]/g)) {
        for (const name of command[1]!.matchAll(/"([^"]+)"/g)) {
          allowedCommands.add(name[1]!);
        }
      }
    }
  } catch {
    // Crate has no examples/permissions directory.
  }
  try {
    for (const file of readdirSync(capabilitiesRoot)) {
      const content = readFileSync(join(capabilitiesRoot, file), "utf8");
      for (const permission of content.matchAll(/"permissions"\s*:\s*\[([^\]]*)\]/g)) {
        for (const name of permission[1]!.matchAll(/"([^"]+)"/g)) {
          if (name[1]!.startsWith("core:")) continue;
          if (!permissionIds.has(name[1]!)) {
            problems.push(
              `${join("crates", directory, "examples", "capabilities", file)} references undeclared permission ${name[1]}`,
            );
          }
        }
      }
    }
  } catch {
    // Crate has no examples/capabilities directory.
  }
}

for (const command of allowedCommands) {
  if (!rustCommands.has(command)) {
    problems.push(`permission allows command with no Rust implementation: ${command}`);
  }
}

if (problems.length > 0) {
  for (const problem of problems) console.error(problem);
  throw new Error(`host protocol drift: ${problems.length} problem(s)`);
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.host-protocol-proof.v1",
      outcome: "pass",
      rustCommands: rustCommands.size,
      rendererCommands: tsCommands.size,
      seamCommands: seamCommands.size + seamCommandsRustOnly.size,
      rustEvents: rustEvents.size,
      rendererEvents: tsEvents.size,
      seamEvents: seamEvents.size + seamEventsRustOnly.size,
      permissions: permissionIds.size,
      allowedCommands: allowedCommands.size,
    },
    null,
    2,
  ),
);
