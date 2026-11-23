import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

interface Diagnostic {
  readonly code: string | null;
  readonly filename: string | null;
  readonly message: string;
  readonly labels: ReadonlyArray<{
    readonly span: { readonly line: number; readonly column: number };
  }>;
}

const root = resolve(import.meta.dir, "..");
const lint = Bun.spawnSync(
  [
    process.execPath,
    "node_modules/.bin/oxlint",
    "--type-aware",
    "--deny-warnings",
    "--format",
    "json",
    "packages",
    "scripts",
    "examples/agent-control-proof",
    "oxlint.config.ts",
    "knip.ts",
  ],
  {
    cwd: root,
    env: { ...process.env, OXLINT_TSGOLINT_PATH: resolve(root, "node_modules/.bin/tsgolint") },
    stdout: "pipe",
    stderr: "inherit",
  },
);
if (lint.exitCode !== 0 && lint.exitCode !== 1) {
  throw new Error(`Oxlint exited with code ${lint.exitCode}`);
}

const output = JSON.parse(lint.stdout.toString()) as {
  readonly diagnostics: ReadonlyArray<Diagnostic>;
};
const sources = new Map<string, ReadonlyArray<string>>();

function fingerprint(diagnostic: Diagnostic): string | undefined {
  const { code, filename, message, labels } = diagnostic;
  const span = labels[0]?.span;
  if (!code || !filename || !span) return undefined;

  let lines = sources.get(filename);
  if (!lines) {
    lines = readFileSync(resolve(root, filename), "utf8").split("\n");
    sources.set(filename, lines);
  }
  const source = lines[span.line - 1]?.trim();
  if (source === undefined) return undefined;

  const hash = createHash("sha256")
    .update(JSON.stringify([code, message, source]))
    .digest("hex");
  return `${filename}:${code}:${hash.slice(0, 16)}`;
}

const diagnostics = output.diagnostics.map((diagnostic) => ({
  diagnostic,
  key: fingerprint(diagnostic),
}));

if (lint.exitCode && output.diagnostics.length === 0) {
  throw new Error("Oxlint failed without diagnostics");
}

if (process.argv.includes("--print-baseline")) {
  // Sort without deduplicating so identical lines retain their exact counts.
  const keys = diagnostics.flatMap(({ key }) => (key === undefined ? [] : [key]));
  console.log(JSON.stringify(keys.sort(), null, 2));
} else {
  const baseline = JSON.parse(
    readFileSync(resolve(root, "config/oxlint-baseline.json"), "utf8"),
  ) as Array<string>;
  let failures = 0;
  for (const { diagnostic, key } of diagnostics) {
    const index = key === undefined ? -1 : baseline.indexOf(key);
    if (index >= 0) {
      baseline.splice(index, 1);
      continue;
    }
    const span = diagnostic.labels[0]?.span;
    const location = span ? `:${span.line}:${span.column}` : "";
    console.error(
      `${diagnostic.filename ?? "oxlint"}${location} ${diagnostic.code ?? ""} ${diagnostic.message}`,
    );
    failures++;
  }

  if (baseline.length > 0) {
    console.error(`Remove ${baseline.length} resolved entries from config/oxlint-baseline.json.`);
    for (const entry of baseline) console.error(entry);
    failures += baseline.length;
  }
  process.exitCode = failures > 0 ? 1 : 0;
}
