// Fails when a manifest or script names a path that resolves above the
// repository root.
//
// Sibling checkouts and machine-local artifact packs made `effigy qa` green
// here while a clean runner failed in seconds. Consumer-isolation catches
// named foreign repositories; this catches the mechanism itself — any
// `path =`, `file:`, or `join(repoRoot, "../…")` that leaves the tree.
//
// Deliberate exception: the greenfield proof packs Poodle from source, so it
// may name a sibling checkout. Nothing else may.

import { readdir, readFile } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

/** Relative paths allowed to name a location outside the repository. */
const ALLOWED_ESCAPES = new Set(["scripts/verify-greenfield-card125.ts"]);

const SKIPPED = new Set([
  "node_modules",
  ".git",
  "target",
  "dist",
  "build",
  ".svelte-kit",
  ".vite",
  ".artifacts",
]);
const SCANNED = [
  "crates",
  "packages",
  "scripts",
  "examples",
  "prototypes",
  "fixtures",
];
const EXTENSIONS = new Set([".ts", ".tsx", ".svelte", ".rs", ".json", ".toml", ".sh"]);

type Finding = { file: string; line: number; detail: string; text: string };
const findings: Finding[] = [];

for (const directory of SCANNED) {
  await scan(join(repoRoot, directory));
}
await inspectManifest(relative(repoRoot, join(repoRoot, "Cargo.toml")), join(repoRoot, "Cargo.toml"));
await inspectManifest(
  relative(repoRoot, join(repoRoot, "package.json")),
  join(repoRoot, "package.json"),
);

if (findings.length > 0) {
  const lines = findings.map(
    ({ file, line, detail, text }) => `  ${file}:${line} ${detail} — ${text}`,
  );
  console.error(
    `Longhorn path references must resolve inside the repository.\n\n${lines.join("\n")}\n\n` +
      "A clean runner has no sibling checkouts and no machine-local packs. The\n" +
      "only admitted escape is scripts/verify-greenfield-card125.ts, which packs\n" +
      "Poodle from source under release.yml.",
  );
  process.exit(1);
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.repo-containment.v1",
      outcome: "pass",
      scanned: SCANNED,
      allowedEscapes: [...ALLOWED_ESCAPES],
    },
    null,
    2,
  ),
);

async function scan(directory: string): Promise<void> {
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (SKIPPED.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      await scan(path);
      continue;
    }
    const extension = entry.name.slice(entry.name.lastIndexOf("."));
    if (!EXTENSIONS.has(extension)) continue;
    if (path === import.meta.path) continue;
    await inspectManifest(relative(repoRoot, path), path);
  }
}

async function inspectManifest(file: string, absolute: string): Promise<void> {
  const source = await readFile(absolute, "utf8");
  if (ALLOWED_ESCAPES.has(file)) return;

  if (file.endsWith("Cargo.toml") || file.endsWith(".toml")) {
    recordPaths(file, source, dirname(absolute), /\bpath\s*=\s*"([^"]+)"/g, "Cargo path escapes the repository");
  }

  if (file.endsWith("package.json")) {
    recordPackagePins(file, source, dirname(absolute));
  }

  // Sibling checkout from the repository root — the shape that made local qa
  // green while CI had nothing to open.
  record(
    file,
    source,
    /\b(?:resolve|join)\(\s*(?:repoRoot|longhornRoot)\s*,\s*["'`]\.\.\/[^"'`]+["'`]/g,
    () => "resolves a path above the repository root",
  );
}

function recordPackagePins(file: string, source: string, base: string): void {
  let manifest: {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    overrides?: Record<string, string | Record<string, string>>;
  };
  try {
    manifest = JSON.parse(source);
  } catch {
    return;
  }

  const pins: Array<[string, string]> = [];
  for (const block of [
    manifest.dependencies,
    manifest.devDependencies,
    manifest.optionalDependencies,
    manifest.peerDependencies,
  ]) {
    if (!block) continue;
    for (const [name, spec] of Object.entries(block)) {
      pins.push([name, spec]);
    }
  }
  if (manifest.overrides) {
    for (const [name, spec] of Object.entries(manifest.overrides)) {
      if (typeof spec === "string") pins.push([name, spec]);
    }
  }

  for (const [name, spec] of pins) {
    const matched = spec.match(/^(file|link):(.+)$/);
    if (!matched) continue;
    const target = matched[2]!;
    const absolute = resolve(base, target);
    if (!escapes(absolute)) continue;
    const line = source.split("\n").findIndex((row) => row.includes(spec)) + 1;
    findings.push({
      file,
      line: line > 0 ? line : 1,
      detail: `${matched[1]}: dependency escapes the repository`,
      text: `${name}: ${spec}`,
    });
  }
}

function recordPaths(
  file: string,
  source: string,
  base: string,
  pattern: RegExp,
  detail: string,
): void {
  pattern.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source)) !== null) {
    const target = match[1]!;
    const absolute = resolve(base, target);
    if (!escapes(absolute)) continue;
    findings.push({
      file,
      line: source.slice(0, match.index).split("\n").length,
      detail,
      text: match[0].trim(),
    });
  }
}

function record(
  file: string,
  source: string,
  pattern: RegExp,
  classify: (match: RegExpExecArray) => string | null,
): void {
  pattern.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source)) !== null) {
    const detail = classify(match);
    if (detail === null) continue;
    findings.push({
      file,
      line: source.slice(0, match.index).split("\n").length,
      detail,
      text: match[0].trim(),
    });
  }
}

function escapes(absolute: string): boolean {
  const rel = relative(repoRoot, absolute);
  return rel === "" ? false : rel.startsWith("..") || isAbsolute(rel);
}
