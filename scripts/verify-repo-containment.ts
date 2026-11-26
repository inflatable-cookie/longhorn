import { parseJsonRecord, requireRecord } from "./proof-json.ts";
import { errorCode, requireValue } from "./proof-support.ts";
// Fails when a manifest or script names a path that resolves above the
// repository root.
//
// Sibling checkouts and machine-local artifact packs made `effigy qa` green
// here while a clean runner failed in seconds. Consumer-isolation catches
// named foreign repositories; this catches the mechanism itself — any
// `path =`, `file:`, or `join(repoRoot, "../…")` that leaves the tree.
//
// Deliberate exception: g02.036 compiles one disposable exact-SHA Swallowtail
// source consumer.

import { readdir, readFile } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";

const defaultRepoRoot = resolve(import.meta.dir, "..");
const selfPath = import.meta.path;

/** Relative paths allowed to name a location outside the repository. */
export const ALLOWED_ESCAPES = new Set(["scripts/verify-agent-tool-dispatch-source-consumer.ts"]);

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
const SCANNED = ["crates", "packages", "scripts", "examples", "prototypes", "fixtures"];
const EXTENSIONS = new Set([".ts", ".tsx", ".svelte", ".rs", ".json", ".toml", ".sh"]);

type Finding = { file: string; line: number; detail: string; text: string };

export async function verifyRepoContainment(repoRoot = defaultRepoRoot) {
  const findings: Array<Finding> = [];

  for (const directory of SCANNED) {
    await scan(repoRoot, join(repoRoot, directory), findings);
  }
  for (const rootManifest of ["Cargo.toml", "package.json"]) {
    const absolute = join(repoRoot, rootManifest);
    try {
      await inspectManifest(repoRoot, relative(repoRoot, absolute), absolute, findings);
    } catch (error) {
      if (errorCode(error) !== "ENOENT") {
        throw error;
      }
    }
  }

  if (findings.length > 0) {
    const lines = findings.map(
      ({ file, line, detail, text }) => `  ${file}:${line} ${detail} — ${text}`,
    );
    throw new Error(
      `Longhorn path references must resolve inside the repository.\n\n${lines.join("\n")}\n\n` +
        "A clean runner has no sibling checkouts and no machine-local packs. The\n" +
        "only admitted escape is g02.036's disposable exact-SHA source-consumer\n" +
        "acceptance runner.",
    );
  }

  return {
    schema: "longhorn.repo-containment.v1",
    outcome: "pass",
    scanned: SCANNED,
    allowedEscapes: [...ALLOWED_ESCAPES],
  };
}

if (import.meta.main) {
  try {
    console.log(JSON.stringify(await verifyRepoContainment(), null, 2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

async function scan(repoRoot: string, directory: string, findings: Array<Finding>): Promise<void> {
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch (error) {
    if (errorCode(error) === "ENOENT") {
      return;
    }
    throw error;
  }
  for (const entry of entries) {
    if (SKIPPED.has(entry.name)) {
      continue;
    }
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      await scan(repoRoot, path, findings);
      continue;
    }
    const extension = entry.name.slice(entry.name.lastIndexOf("."));
    if (!EXTENSIONS.has(extension)) {
      continue;
    }
    if (path === selfPath) {
      continue;
    }
    await inspectManifest(repoRoot, relative(repoRoot, path), path, findings);
  }
}

async function inspectManifest(
  repoRoot: string,
  file: string,
  absolute: string,
  findings: Array<Finding>,
): Promise<void> {
  const source = await readFile(absolute, "utf8");
  if (ALLOWED_ESCAPES.has(file)) {
    return;
  }

  if (file.endsWith("Cargo.toml") || file.endsWith(".toml")) {
    recordPaths(
      repoRoot,
      file,
      source,
      dirname(absolute),
      /\bpath\s*=\s*"([^"]+)"/g,
      "Cargo path escapes the repository",
      findings,
    );
  }

  if (file.endsWith("package.json")) {
    recordPackagePins(repoRoot, file, source, dirname(absolute), findings);
  }

  // Sibling checkout from the repository root — the shape that made local qa
  // green while CI had nothing to open.
  record(
    file,
    source,
    /\b(?:resolve|join)\(\s*(?:repoRoot|longhornRoot)\s*,\s*["'`]\.\.\/[^"'`]+["'`]/g,
    () => "resolves a path above the repository root",
    findings,
  );
}

function recordPackagePins(
  repoRoot: string,
  file: string,
  source: string,
  base: string,
  findings: Array<Finding>,
): void {
  const manifest = parseJsonRecord(source);

  const pins: Array<[string, string]> = [];
  for (const block of [
    manifest.dependencies,
    manifest.devDependencies,
    manifest.optionalDependencies,
    manifest.peerDependencies,
  ]) {
    if (!block) {
      continue;
    }
    for (const [name, spec] of Object.entries(requireRecord(block, "dependency table"))) {
      if (typeof spec === "string") {
        pins.push([name, spec]);
      }
    }
  }
  if (manifest.overrides) {
    for (const [name, spec] of Object.entries(requireRecord(manifest.overrides, "overrides"))) {
      if (typeof spec === "string") {
        pins.push([name, spec]);
      }
    }
  }

  for (const [name, spec] of pins) {
    const matched = spec.match(/^(file|link):(.+)$/);
    if (!matched) {
      continue;
    }
    const target = requireValue(matched[2], "matched[2]");
    const absolute = resolve(base, target);
    if (!escapes(repoRoot, absolute)) {
      continue;
    }
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
  repoRoot: string,
  file: string,
  source: string,
  base: string,
  pattern: RegExp,
  detail: string,
  findings: Array<Finding>,
): void {
  pattern.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source)) !== null) {
    const target = requireValue(match[1], "match[1]");
    const absolute = resolve(base, target);
    if (!escapes(repoRoot, absolute)) {
      continue;
    }
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
  findings: Array<Finding>,
): void {
  pattern.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source)) !== null) {
    const detail = classify(match);
    if (detail === null) {
      continue;
    }
    findings.push({
      file,
      line: source.slice(0, match.index).split("\n").length,
      detail,
      text: match[0].trim(),
    });
  }
}

function escapes(repoRoot: string, absolute: string): boolean {
  const rel = relative(repoRoot, absolute);
  return rel === "" ? false : rel.startsWith("..") || isAbsolute(rel);
}
