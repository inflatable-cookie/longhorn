import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

// `verify-host-protocol.ts` answers "does every name have an implementation
// and a consumer somewhere". This script answers the narrower question the
// longhorn-tauri raw ports pose: each TS port file hand-writes the invoke and
// event strings of exactly one `longhorn-tauri-*` crate, and the two must
// carry exactly the same strings. The port tests use the exported constants on
// both sides of the fake transport, so a typo there passes everything; only a
// cross-language comparison catches it.

// TS port files whose strings have no Rust implementation in this repo by
// contract. Keyed by file name, valued by the reason, so a new orphan file
// fails here until someone writes down why it is allowed.
const consumerImplementedPorts = new Map([
  [
    "native-content",
    "consumer-implemented host; contract 017 native-content islands (see verify-host-protocol.ts)",
  ],
]);

// `longhorn-tauri-*` crates whose seam strings no `packages/longhorn-tauri`
// port mirrors. Their names live in the peerless package (`src/<domain>/names.ts`
// and friends) because the renderer wiring is consumer-filled. Same rule: a
// new crate without a port fails until its reason is recorded here.
const cratesWithoutPorts = new Map([
  ["command", "contract 010; renderer catalogue/keymap wiring is consumer-filled"],
  ["config", "contract 004/010; storage, backup and restore wiring is consumer-filled"],
  ["settings", "contract 005/010; settings wiring is consumer-filled"],
  ["transfer", "contract 011; transfer wiring is consumer-filled"],
  ["windowing", "window host wiring is consumer-filled"],
  [
    "native-content-child-view",
    "child-view adapter exposes no `#[tauri::command]` seam strings",
  ],
]);

function walk(directory: string, suffix: string, files: string[] = []): string[] {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) walk(path, suffix, files);
    else if (path.endsWith(suffix)) files.push(path);
  }
  return files;
}

// Rust side, per crate: `#[tauri::command]` function names (the invoke name is
// the function name) plus `pub const ..: &str = "longhorn://.."` event names.
function rustSeamStrings(crateDirectory: string): Set<string> {
  const found = new Set<string>();
  for (const file of walk(crateDirectory, ".rs")) {
    const text = readFileSync(file, "utf8");
    for (const match of text.matchAll(
      /#\[tauri::command[^\]]*\]\s*(?:pub\s+)?(?:async\s+)?fn\s+(longhorn_[a-z0-9_]+)/gs,
    )) {
      found.add(match[1]!);
    }
    for (const match of text.matchAll(/pub const [A-Z0-9_]+: &str = "(longhorn:\/\/[^"]+)"/g)) {
      found.add(match[1]!);
    }
  }
  return found;
}

// TS side, per file: every `export const NAME = "..."` string literal, across
// the line break the formatter puts between `=` and the string.
function tsSeamStrings(file: string): Set<string> {
  const text = readFileSync(file, "utf8");
  const found = new Set<string>();
  for (const match of text.matchAll(/export const [A-Z0-9_]+ =\s*"([^"]+)";/gs)) {
    found.add(match[1]!);
  }
  return found;
}

const crateDomains = readdirSync(join(repoRoot, "crates"))
  .filter((name) => name.startsWith("longhorn-tauri-"))
  .map((name) => name.slice("longhorn-tauri-".length))
  // Longest first, so `history-tree.ts` pairs with the history-tree crate and
  // not with history.
  .sort((a, b) => b.length - a.length);

const portFiles = readdirSync(join(repoRoot, "packages/longhorn-tauri/src"), {
  withFileTypes: true,
})
  .filter((entry) => entry.isFile() && entry.name.endsWith(".ts"))
  .map((entry) => entry.name.slice(0, -".ts".length))
  .sort();

// Assign each port file to the longest crate domain it prefixes
// (`bridge-events` belongs to bridge). `undefined` means no crate claims it.
const portByDomain = new Map<string, string[]>();
const unpairedPorts: string[] = [];
for (const file of portFiles) {
  const domain = crateDomains.find((name) => file === name || file.startsWith(`${name}-`));
  if (domain === undefined) {
    unpairedPorts.push(file);
  } else {
    portByDomain.set(domain, [...(portByDomain.get(domain) ?? []), file]);
  }
}

const problems: string[] = [];

for (const file of unpairedPorts) {
  if (!consumerImplementedPorts.has(file)) {
    problems.push(
      `packages/longhorn-tauri/src/${file}.ts has no matching longhorn-tauri-* crate; ` +
        `pair it or record why its seam is consumer-implemented`,
    );
  }
}

for (const domain of crateDomains) {
  const files = portByDomain.get(domain);
  const rust = rustSeamStrings(join(repoRoot, "crates", `longhorn-tauri-${domain}`, "src"));
  if (files === undefined) {
    if (rust.size > 0 && !cratesWithoutPorts.has(domain)) {
      problems.push(
        `crates/longhorn-tauri-${domain} declares ${rust.size} seam string(s) no ` +
          `packages/longhorn-tauri port mirrors; add the port or record why the domain has none`,
      );
    }
    continue;
  }
  const ts = new Set<string>();
  for (const file of files) {
    for (const value of tsSeamStrings(join(repoRoot, "packages/longhorn-tauri/src", `${file}.ts`))) {
      ts.add(value);
    }
  }
  for (const value of rust) {
    if (!ts.has(value)) {
      problems.push(`${domain}: Rust declares "${value}" but no TS port exports it`);
    }
  }
  for (const value of ts) {
    if (!rust.has(value)) {
      problems.push(`${domain}: TS exports "${value}" but crates/longhorn-tauri-${domain} never declares it`);
    }
  }
}

if (problems.length > 0) {
  for (const problem of problems) console.error(problem);
  throw new Error(`tauri seam-string drift: ${problems.length} problem(s)`);
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.tauri-seam-strings.v1",
      outcome: "pass",
      pairedDomains: [...portByDomain.keys()].sort(),
      consumerImplementedPorts: consumerImplementedPorts.size,
      cratesWithoutPorts: cratesWithoutPorts.size,
    },
    null,
    2,
  ),
);
