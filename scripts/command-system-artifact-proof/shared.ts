import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

export async function digest(path: string): Promise<string> {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
}

export async function run(
  command: readonly string[],
  cwd: string,
): Promise<string> {
  const process = Bun.spawn(command, {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ]);
  if (exitCode !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`,
    );
  }
  return `${stdout}\n${stderr}`;
}

export function fileDependency(path: string): string {
  return `file:${path}`;
}

export function assertExactSet(
  label: string,
  actual: readonly string[],
  expected: readonly string[],
): void {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) {
    throw new Error(
      `${label} mismatch: ${left.join(", ")} != ${right.join(", ")}`,
    );
  }
}

export function parseTrace(output: string): Record<string, unknown> {
  const line = output
    .split("\n")
    .find((candidate) => candidate.trim().startsWith("{"));
  if (line === undefined) throw new Error("proof emitted no JSON trace");
  return JSON.parse(line) as Record<string, unknown>;
}

export function testCount(output: string): number {
  const match = output.match(/Tests\s+(\d+) passed/);
  return match ? Number(match[1]) : 0;
}
