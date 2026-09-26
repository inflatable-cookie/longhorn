// Prove `effigy release:gates` is `[release.gates]` minus `workspace`, in
// declaration order. The runner runs `effigy qa` separately; this check
// refuses a silent subset.

import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const defaultRepoRoot = resolve(import.meta.dir, "..");

export type GateLine = {
  readonly name: string;
  readonly command: string;
};

export type AlignmentResult = {
  readonly expected: readonly string[];
  readonly actual: readonly string[];
};

export function parseReleaseGates(toml: string): GateLine[] {
  const header = /^\[release\.gates\]\s*$/m.exec(toml);
  if (!header || header.index === undefined) {
    throw new Error("config/release.toml has no [release.gates] table");
  }
  const start = header.index + header[0].length;
  const body = toml.slice(start);
  const end = body.search(/\n\[/m);
  const table = end === -1 ? body : body.slice(0, end);
  const gates: GateLine[] = [];
  for (const raw of table.split("\n")) {
    const line = raw.trim();
    if (line.length === 0 || line.startsWith("#")) continue;
    const match = /^([A-Za-z0-9_-]+)\s*=\s*"([^"]+)"$/.exec(line);
    if (!match) {
      throw new Error(`unparseable [release.gates] line: ${raw}`);
    }
    gates.push({ name: match[1], command: match[2] });
  }
  if (gates.length === 0) throw new Error("[release.gates] is empty");
  return gates;
}

export function parseReleaseGatesSelector(toml: string): string[] {
  const match = /"release:gates"\s*=\s*\[([\s\S]*?)\]/.exec(toml);
  if (!match?.[1]) throw new Error('effigy.toml has no "release:gates" array');
  const commands: string[] = [];
  for (const raw of match[1].split("\n")) {
    const line = raw.trim().replace(/,$/, "");
    if (line.length === 0 || line.startsWith("#")) continue;
    const task = /^\{\s*task\s*=\s*"([^"]+)"\s*\}$/.exec(line);
    if (task) {
      commands.push(`effigy ${task[1]}`);
      continue;
    }
    const literal = /^"([^"]+)"$/.exec(line);
    if (literal) {
      commands.push(literal[1]);
      continue;
    }
    throw new Error(`unparseable release:gates entry: ${raw}`);
  }
  if (commands.length === 0) throw new Error("release:gates is empty");
  return commands;
}

export function alignReleaseGates(releaseToml: string, effigyToml: string): AlignmentResult {
  const expected = parseReleaseGates(releaseToml)
    .filter((gate) => gate.name !== "workspace")
    .map((gate) => gate.command);
  const actual = parseReleaseGatesSelector(effigyToml);
  return { expected, actual };
}

export function alignmentFailures(result: AlignmentResult): string[] {
  const failures: string[] = [];
  if (result.expected.length !== result.actual.length) {
    failures.push(
      `release:gates has ${result.actual.length} entries; [release.gates] minus workspace has ${result.expected.length}`,
    );
  }
  const length = Math.max(result.expected.length, result.actual.length);
  for (let index = 0; index < length; index += 1) {
    const expected = result.expected[index];
    const actual = result.actual[index];
    if (expected !== actual) {
      failures.push(`index ${index}: expected ${expected ?? "<missing>"}, actual ${actual ?? "<missing>"}`);
    }
  }
  return failures;
}

export function checkReleaseGatesAlignment(repoRoot = defaultRepoRoot): AlignmentResult {
  const releaseToml = readFileSync(join(repoRoot, "config/release.toml"), "utf8");
  const effigyToml = readFileSync(join(repoRoot, "effigy.toml"), "utf8");
  const result = alignReleaseGates(releaseToml, effigyToml);
  const failures = alignmentFailures(result);
  if (failures.length > 0) {
    throw new Error(`release:gates is not [release.gates] minus workspace:\n${failures.join("\n")}`);
  }
  return result;
}

if (import.meta.main) {
  const result = checkReleaseGatesAlignment();
  console.log(JSON.stringify({
    schema: "longhorn.release-gates-alignment.v1",
    outcome: "pass",
    gates: result.actual,
  }, null, 2));
}
