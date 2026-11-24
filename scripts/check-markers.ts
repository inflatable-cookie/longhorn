// Attention markers, the retired `scan attention-markers` rules ported to a gate.
//
// Effigy 0.15 removed the scanner and its `[scan.attention_markers]` table. The
// marker lists below are the ones that table set in this repository. The rules
// are the scanner's, as built in stable 8a58403:
//   1. Scope: tracked text files with a code extension, outside excluded
//      directories, documentation, data and lockfile paths.
//   2. Generated artifacts are skipped: a `.min.` or `.generated.rs` name, or a
//      generated header in the first 24 lines.
//   3. A marker matches a line when, with string literals masked, the lowercased
//      line contains it and the marker starts a comment body: after a comment
//      prefix, or after the first inline `//`, `/*` or `<!--`. The marker must
//      end the word: end of line, `:`, space, `]`, `)` or `-` follows it.
//      A marker starting with `@` also matches a `#[deprecated` line, and so
//      does any marker containing `deprecat`.
//   4. A marker counts once per line; a leading `@` does not separate markers.
//   5. Category: deprecation, temporary artifact, or deferred work.
//   6. Severity comes from the list the marker sits in.
// The gate fails on critical markers. High and warning matches are reported as
// counts only.

import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { hasGeneratedHeader, maskStringLiterals } from "./scanner-heuristics.ts";

const repoRoot = resolve(import.meta.dir, "..");

export type AttentionSeverity = "critical" | "high" | "warning";
export type AttentionCategory = "deprecation" | "temporary-artifact" | "deferred-work";

export const ATTENTION_PATTERNS = {
  critical: ["BUG", "remove before release"],
  high: ["FIXME", "HACK", "@deprecated", "workaround"],
  warning: ["TODO", "REVIEW", "NOTE", "placeholder"],
} satisfies Record<AttentionSeverity, ReadonlyArray<string>>;

const SEVERITY_ORDER: ReadonlyArray<AttentionSeverity> = ["critical", "high", "warning"];

// Extensions the scanner treated as code. Anything else is not scanned.
const CODE_EXTENSIONS: ReadonlySet<string> = new Set([
  "c",
  "cc",
  "cpp",
  "cs",
  "css",
  "go",
  "h",
  "hpp",
  "java",
  "js",
  "jsx",
  "kt",
  "kts",
  "lua",
  "m",
  "mm",
  "php",
  "py",
  "rb",
  "rs",
  "scala",
  "sc",
  "sh",
  "sql",
  "swift",
  "ts",
  "tsx",
  "vue",
  "zsh",
]);
const EXCLUDED_DIRS: ReadonlySet<string> = new Set([
  ".git",
  ".next",
  ".nuxt",
  ".svelte-kit",
  ".turbo",
  ".yarn",
  "build",
  "coverage",
  "dist",
  "node_modules",
  "target",
  "vendor",
]);
const DOC_DIRS: ReadonlySet<string> = new Set(["doc", "docs", "documentation"]);
const DATA_DIRS: ReadonlySet<string> = new Set([
  "benchmark",
  "benchmarks",
  "benches",
  "example",
  "examples",
  "fixture",
  "fixtures",
  "migration",
  "migrations",
  "migrations_dev",
  "seeds",
]);
const LOCK_FILES: ReadonlySet<string> = new Set([
  "package-lock.json",
  "pnpm-lock.yaml",
  "yarn.lock",
  "cargo.lock",
  "bun.lock",
  "bun.lockb",
  "composer.lock",
]);

const TEMPORARY_WORDS: ReadonlyArray<string> = [
  "temporary",
  "workaround",
  "remove before",
  "placeholder",
  "stub",
  "later",
  "tech debt",
];

export type AttentionFinding = {
  readonly path: string;
  readonly line: number;
  readonly marker: string;
  readonly severity: AttentionSeverity;
  readonly category: AttentionCategory;
  readonly snippet: string;
};

// Rule 1: the scanner's scope for attention markers.
export function isAttentionScanPath(path: string): boolean {
  const components = path.split("/");
  if (components.some((c) => EXCLUDED_DIRS.has(c) || DOC_DIRS.has(c) || DATA_DIRS.has(c))) {
    return false;
  }
  const name = components.at(-1) ?? "";
  if (LOCK_FILES.has(name.toLowerCase())) {
    return false;
  }
  const dot = name.lastIndexOf(".");
  const extension = dot > 0 ? name.slice(dot + 1) : "";
  return CODE_EXTENSIONS.has(extension);
}

// Rule 2: the scanner's generated-artifact skip for attention markers.
export function isGeneratedArtifact(path: string, text: string): boolean {
  const name = (path.split("/").at(-1) ?? "").toLowerCase();
  if (name.includes(".min.") || name.endsWith(".generated.rs")) {
    return true;
  }
  return hasGeneratedHeader(text);
}

// Rule 3: whether a marker (already lowercased) matches one raw line.
export function attentionLineMatches(rawLine: string, markerLower: string): boolean {
  const lower = maskStringLiterals(rawLine).toLowerCase();
  if (!lower.includes(markerLower)) {
    return false;
  }
  const commentMatch = commentStartsWithMarker(lower, markerLower);
  if (markerLower.startsWith("@")) {
    return commentMatch || lower.includes("#[deprecated");
  }
  if (markerLower.includes("deprecat")) {
    return lower.includes("#[deprecated") || commentMatch;
  }
  return commentMatch;
}

function commentStartsWithMarker(lower: string, markerLower: string): boolean {
  const body = stripCommentPrefix(lower.trimStart());
  if (body !== undefined && bodyStartsWithMarker(body, markerLower)) {
    return true;
  }
  const inline = inlineCommentBody(lower);
  return inline !== undefined && bodyStartsWithMarker(inline, markerLower);
}

// The text after the first `//`, `/*` or `<!--` whose remainder is not empty.
function inlineCommentBody(lower: string): string | undefined {
  for (const prefix of ["//", "/*", "<!--"]) {
    const index = lower.indexOf(prefix);
    if (index !== -1) {
      const candidate = lower.slice(index + prefix.length);
      if (candidate !== "") {
        return candidate;
      }
    }
  }
  return undefined;
}

function stripCommentPrefix(trimmed: string): string | undefined {
  for (const prefix of ["//", "/*", "*", "<!--", "--"]) {
    if (trimmed.startsWith(prefix)) {
      return trimmed.slice(prefix.length);
    }
  }
  if (trimmed.startsWith("#[") || trimmed.startsWith("##")) {
    return undefined;
  }
  return trimmed.startsWith("#") ? trimmed.slice(1) : undefined;
}

function bodyStartsWithMarker(body: string, markerLower: string): boolean {
  let rest = body.trimStart();
  while (rest.length > 0 && "-*[(".includes(rest[0] ?? "")) {
    rest = rest.slice(1);
  }
  rest = rest.trimStart();
  if (!rest.startsWith(markerLower)) {
    return false;
  }
  const after = rest.slice(markerLower.length);
  return after === "" || [":", " ", "]", ")", "-"].some((end) => after.startsWith(end));
}

// Rule 5: the scanner's category for a lowercased marker.
export function attentionCategory(markerLower: string): AttentionCategory {
  if (markerLower.includes("deprecat")) {
    return "deprecation";
  }
  if (TEMPORARY_WORDS.some((word) => markerLower.includes(word))) {
    return "temporary-artifact";
  }
  return "deferred-work";
}

// Rules 3, 4, 6 for one raw line, in the scanner's order: critical, high, warning.
export function attentionLineFindings(
  path: string,
  line: number,
  rawLine: string,
): Array<AttentionFinding> {
  const snippet = rawLine.trim();
  if (snippet === "") {
    return [];
  }
  const findings: Array<AttentionFinding> = [];
  const seen = new Set<string>();
  for (const severity of SEVERITY_ORDER) {
    for (const marker of ATTENTION_PATTERNS[severity]) {
      const markerLower = marker.toLowerCase();
      if (!attentionLineMatches(rawLine, markerLower)) {
        continue;
      }
      const key = markerLower.replace(/^@+/, "");
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      findings.push({
        path,
        line,
        marker,
        severity,
        category: attentionCategory(markerLower),
        snippet: snippet.slice(0, 120),
      });
    }
  }
  return findings;
}

// Every finding in one file, after the scope and generated-artifact rules.
export function attentionFileFindings(path: string, text: string): Array<AttentionFinding> {
  if (!isAttentionScanPath(path) || isGeneratedArtifact(path, text)) {
    return [];
  }
  const findings: Array<AttentionFinding> = [];
  text.split(/\r?\n/).forEach((rawLine, index) => {
    findings.push(...attentionLineFindings(path, index + 1, rawLine));
  });
  return findings;
}

function trackedFiles(): Array<string> {
  const result = spawnSync("git", ["ls-files"], { cwd: repoRoot, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`git ls-files failed: ${result.stderr || String(result.error)}`);
  }
  return result.stdout.split("\n").filter((path) => path !== "");
}

export async function gatherAttentionFindings(): Promise<Array<AttentionFinding>> {
  const paths = trackedFiles().filter(isAttentionScanPath);
  const perFile = await Promise.all(
    paths.map(async (path) => {
      const text = await readFile(resolve(repoRoot, path), "utf8");
      return attentionFileFindings(path, text);
    }),
  );
  return perFile.flat();
}

if (import.meta.main) {
  const findings = await gatherAttentionFindings();
  const critical = findings.filter((finding) => finding.severity === "critical");
  const high = findings.filter((finding) => finding.severity === "high").length;
  const warning = findings.filter((finding) => finding.severity === "warning").length;
  if (critical.length > 0) {
    for (const finding of critical) {
      console.error(
        `check:markers: ${finding.path}:${finding.line}: critical attention marker ${finding.marker}: ${finding.snippet}`,
      );
    }
    process.exit(1);
  }
  console.log(
    `check:markers: no critical markers; high ${high} and warning ${warning} findings (report only)`,
  );
}
