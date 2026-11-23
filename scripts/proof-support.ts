import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

export function sha256(value: string | Uint8Array): string {
  return createHash("sha256").update(value).digest("hex");
}

export async function digest(path: string): Promise<string> {
  return sha256(await readFile(path));
}

export function fileDependency(path: string): string {
  return `file:${path}`;
}

export function assertExactSet(
  label: string,
  actual: ReadonlyArray<string>,
  expected: ReadonlyArray<string>,
): void {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) {
    throw new Error(`${label} mismatch: ${JSON.stringify(left)} != ${JSON.stringify(right)}`);
  }
}

type RunOptions = {
  env?: Record<string, string>;
  includeStderr?: boolean;
};

export async function run(
  command: ReadonlyArray<string>,
  cwd: string,
  { env, includeStderr = false }: RunOptions = {},
): Promise<string> {
  const child = Bun.spawn([...command], {
    cwd,
    env: { ...process.env, ...env },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (exitCode !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd} (exit ${exitCode})\n${stdout}\n${stderr}`,
    );
  }
  return includeStderr ? `${stdout}\n${stderr}` : stdout;
}

export function runWithDiagnostics(command: ReadonlyArray<string>, cwd: string): Promise<string> {
  return run(command, cwd, { includeStderr: true });
}

export function runArtifactCommand(command: ReadonlyArray<string>, cwd: string): Promise<string> {
  return run(command, cwd, { includeStderr: true, env: { COPYFILE_DISABLE: "1" } });
}

export function runSync(command: ReadonlyArray<string>, cwd: string): string {
  const [program, ...args] = command;
  if (!program) {
    throw new Error("proof command is empty");
  }
  const result = spawnSync(program, args, {
    cwd,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd} (exit ${result.status})\n${result.stdout}\n${result.stderr}`,
    );
  }
  return result.stdout;
}

export function parseTrace(output: string): Record<string, unknown> {
  const line = output.split("\n").find((candidate) => candidate.trim().startsWith("{"));
  if (line === undefined) {
    throw new Error("proof emitted no JSON trace");
  }
  const trace: unknown = JSON.parse(line);
  if (trace === null || typeof trace !== "object" || Array.isArray(trace)) {
    throw new Error("proof trace must be a JSON object");
  }
  return Object.fromEntries(Object.entries(trace));
}
