// Fails when scripts invoke tools a clean GitHub runner does not have.
//
// Release run 6 died on missing `rg` twice: once in the greenfield proof, once
// in check-release-floor.sh, where `set -o pipefail` made the absence look
// like a missing MSRV toolchain. Local machines usually have ripgrep, so qa
// never saw it.
//
// Comments may mention rg. Command invocations may not.

import { readdir, readFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const SKIPPED = new Set([
  "node_modules",
  ".git",
  "target",
  "dist",
  "build",
  ".svelte-kit",
  ".vite",
]);
const ROOTS = ["scripts", ".github", "config"];
const EXTENSIONS = new Set([".ts", ".tsx", ".js", ".mjs", ".cjs", ".sh", ".yml", ".yaml", ".toml"]);

// Word-boundary forms that are invocations, not prose. Comments are stripped
// before matching so "prefer grep, not rg" in a header stays allowed.
const FORBIDDEN = [
  { tool: "rg", pattern: /(?:^|[\s|`$])rg(?:\s|["'`|]|$)/ },
  { tool: "ripgrep", pattern: /(?:^|[\s|`$])ripgrep(?:\s|["'`|]|$)/ },
];

type Finding = { file: string; line: number; tool: string; text: string };
const findings: Finding[] = [];

for (const root of ROOTS) {
  await scan(join(repoRoot, root));
}
// The task surface itself: task commands invoke tools too.
inspect("effigy.toml", await readFile(join(repoRoot, "effigy.toml"), "utf8"));

if (findings.length > 0) {
  const lines = findings.map(
    ({ file, line, tool, text }) => `  ${file}:${line} invokes '${tool}' — ${text}`,
  );
  console.error(
    `Scripts must not invoke tools a clean runner lacks.\n\n${lines.join("\n")}\n\n` +
      "Use grep, or scan in-process. Installing the tool in the workflow fixes\n" +
      "one run and leaves the next absent tool invisible locally.",
  );
  process.exit(1);
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.runner-tools.v1",
      outcome: "pass",
      forbidden: FORBIDDEN.map((item) => item.tool),
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
    const extension = entry.name.includes(".")
      ? entry.name.slice(entry.name.lastIndexOf("."))
      : "";
    if (!EXTENSIONS.has(extension)) continue;
    if (path === import.meta.path) continue;
    inspect(relative(repoRoot, path), await readFile(path, "utf8"));
  }
}

function inspect(file: string, source: string): void {
  const lines = source.split("\n");
  for (let index = 0; index < lines.length; index += 1) {
    const raw = lines[index]!;
    const code = stripComment(file, raw);
    if (!code.trim()) continue;
    for (const { tool, pattern } of FORBIDDEN) {
      if (!pattern.test(code)) continue;
      findings.push({
        file,
        line: index + 1,
        tool,
        text: raw.trim(),
      });
    }
  }
}

function stripComment(file: string, line: string): string {
  if (file.endsWith(".sh")) {
    return line.replace(/(^|\s)#.*$/, "$1");
  }
  if (file.endsWith(".yml") || file.endsWith(".yaml")) {
    return line.replace(/(^|\s)#.*$/, "$1");
  }
  // TypeScript / JS: line comments only. Block comments naming rg in a
  // paragraph are rare in scripts; keep the checker dumb and fast.
  return line.replace(/\/\/.*$/, "").replace(/\/\*.*?\*\//g, "");
}
