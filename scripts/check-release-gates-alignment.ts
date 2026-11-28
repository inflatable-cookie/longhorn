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
  readonly expected: ReadonlyArray<string>;
  readonly actual: ReadonlyArray<string>;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// Gate commands are read as TOML values, so quotes, brackets and `#` inside a
// command are the command's own text, not syntax the reader has to guess at.
export function parseReleaseGates(toml: string): Array<GateLine> {
  const parsed: unknown = Bun.TOML.parse(toml);
  const release = isRecord(parsed) ? parsed["release"] : undefined;
  const table = isRecord(release) ? release["gates"] : undefined;
  if (!isRecord(table)) {
    throw new Error("config/release.toml has no [release.gates] table");
  }
  const gates = Object.entries(table).map(([name, command]) => {
    if (typeof command !== "string" || command.trim() === "") {
      throw new Error(`[release.gates] ${name} must be a non-empty command string`);
    }
    return { name, command };
  });
  if (gates.length === 0) {
    throw new Error("[release.gates] is empty");
  }
  return gates;
}

export function parseReleaseGatesSelector(toml: string): Array<string> {
  const parsed: unknown = Bun.TOML.parse(toml);
  const tasks = isRecord(parsed) ? parsed["tasks"] : undefined;
  const entries = isRecord(tasks) ? tasks["release:gates"] : undefined;
  if (!Array.isArray(entries)) {
    throw new Error('effigy.toml has no "release:gates" array');
  }
  const commands = entries.map((entry: unknown): string => {
    if (typeof entry === "string" && entry.trim() !== "") {
      return entry;
    }
    if (isRecord(entry) && typeof entry["task"] === "string" && entry["task"] !== "") {
      return `effigy ${entry["task"]}`;
    }
    throw new Error(`unparseable release:gates entry: ${JSON.stringify(entry)}`);
  });
  if (commands.length === 0) {
    throw new Error("release:gates is empty");
  }
  return commands;
}

export function alignReleaseGates(releaseToml: string, effigyToml: string): AlignmentResult {
  const expected = parseReleaseGates(releaseToml)
    .filter((gate) => gate.name !== "workspace")
    .map((gate) => gate.command);
  const actual = parseReleaseGatesSelector(effigyToml);
  return { expected, actual };
}

export function alignmentFailures(result: AlignmentResult): Array<string> {
  const failures: Array<string> = [];
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
      failures.push(
        `index ${index}: expected ${expected ?? "<missing>"}, actual ${actual ?? "<missing>"}`,
      );
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
    throw new Error(
      `release:gates is not [release.gates] minus workspace:\n${failures.join("\n")}`,
    );
  }
  return result;
}

if (import.meta.main) {
  const result = checkReleaseGatesAlignment();
  console.log(
    JSON.stringify(
      {
        schema: "longhorn.release-gates-alignment.v1",
        outcome: "pass",
        gates: result.actual,
      },
      null,
      2,
    ),
  );
}
