// Fails when Longhorn code gains direct knowledge of another repository.
//
// The rule is narrow and structural: no file may reach into a sibling checkout
// that is not a declared dependency, whether by absolute path, by `../name`
// from the repository root, or by a `*_REPO` environment override.
//
// Consumer *names* are deliberately not checked. A test graph shaped like a
// real application, or an example named for the topology it exercises, is
// vocabulary. What is not allowed is a path, because a proof that reads a
// second repository can only be green when both repositories are
// simultaneously in one particular state.
//
// No consumer is named here, so a consumer arriving or leaving never requires
// editing this file — which is the failure the previous arrangement had, where
// a hardcoded inventory was edited because one consumer had been renamed.
//
// Twenty-three verifiers violating this were deleted on 2026-08-10, along with
// the twenty-one frozen receipts they asserted against.

import { readdir, readFile } from "node:fs/promises";
import { basename, join, relative, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

/**
 * Sibling repositories Longhorn depends on, rather than applications that
 * depend on Longhorn. Poodle is projected into by the binding tier, so the
 * artifact proofs legitimately read it.
 *
 * Adding an entry should be argued for. The value of a short list is that
 * growing it is visible in review.
 */
const DEPENDENCIES = new Set(["poodle", "poodle-specs"]);

/** This repository, which is not a sibling of itself. */
const SELF = "longhorn";

const SKIPPED = new Set([
  "node_modules", ".git", "target", "dist", "build", ".svelte-kit", ".vite",
]);
const SCANNED = ["crates", "packages", "scripts", "examples", "prototypes", "fixtures"];
const EXTENSIONS = new Set([".ts", ".tsx", ".svelte", ".rs", ".json", ".toml", ".sh"]);

type Finding = { file: string; line: number; detail: string; text: string };
const findings: Finding[] = [];

for (const directory of SCANNED) {
  await scan(join(repoRoot, directory));
}

if (findings.length > 0) {
  const lines = findings.map(
    ({ file, line, detail, text }) => `  ${file}:${line} ${detail} — ${text}`,
  );
  console.error(
    `Longhorn keeps no code with direct knowledge of another repository.\n\n${lines.join("\n")}\n\n` +
      "A name is vocabulary and is fine; a path is not. If the repository is a\n" +
      "dependency rather than a consumer, add it to DEPENDENCIES in this file.",
  );
  process.exit(1);
}

console.log(JSON.stringify({
  schema: "longhorn.consumer-isolation.v1",
  outcome: "pass",
  scanned: SCANNED,
  dependencies: [...DEPENDENCIES],
}, null, 2));

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
    // this file quotes the patterns it forbids
    if (path === import.meta.path) continue;

    inspect(relative(repoRoot, path), await readFile(path, "utf8"));
  }
}

function inspect(file: string, source: string): void {
  record(file, source, /process\.env\.([A-Z0-9_]+)_REPO\b/g, (match) =>
    DEPENDENCIES.has(match[1]!.toLowerCase().replaceAll("_", "-"))
      ? null
      : "takes a sibling-repository environment override",
  );

  // `../name` from the repository root, and absolute paths into a checkout
  record(file, source, /\bprojects\/([a-z0-9-]+)\//g, (match) =>
    match[1] === SELF || DEPENDENCIES.has(match[1]!)
      ? null
      : "hardcodes a path into another checkout",
  );

  record(file, source, /\b(?:resolve|join)\(\s*(?:repoRoot|longhornRoot)\s*,\s*["'`]\.\.\/([a-z0-9-]+)/g,
    (match) => DEPENDENCIES.has(match[1]!) ? null : "resolves a sibling checkout",
  );
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
