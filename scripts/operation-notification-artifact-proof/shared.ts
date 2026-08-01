import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";

export async function digest(path: string): Promise<string> {
  return createHash("sha256").update(await readFile(path)).digest("hex");
}

export async function run(command: readonly string[], cwd: string): Promise<string> {
  const subprocess = Bun.spawn(command, { cwd, env: { ...process.env, COPYFILE_DISABLE: "1" }, stdout: "pipe", stderr: "pipe" });
  const [exitCode, stdout, stderr] = await Promise.all([subprocess.exited, new Response(subprocess.stdout).text(), new Response(subprocess.stderr).text()]);
  if (exitCode !== 0) throw new Error(`${command.join(" ")} failed in ${cwd}\n${stdout}\n${stderr}`);
  return `${stdout}\n${stderr}`;
}

export function fileDependency(path: string): string {
  return `file:${path}`;
}

export function assertExactSet(label: string, actual: readonly string[], expected: readonly string[]): void {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) throw new Error(`${label} mismatch: ${left.join(", ")} != ${right.join(", ")}`);
}

export function parseTrace(output: string): Record<string, unknown> {
  const line = output.split("\n").find((candidate) => candidate.trim().startsWith("{"));
  if (line === undefined) throw new Error("proof emitted no JSON trace");
  return JSON.parse(line) as Record<string, unknown>;
}

export function testCount(output: string): number {
  const match = output.match(/Tests\s+(\d+) passed/);
  return match ? Number(match[1]) : 0;
}

export function equalJson(left: unknown, right: unknown): boolean {
  return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right));
}

function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === "object") return Object.fromEntries(Object.entries(value).sort(([left], [right]) => left.localeCompare(right)).map(([key, entry]) => [key, canonical(entry)]));
  return value;
}

export async function readSourceTree(root: string): Promise<string> {
  const files = (await readdir(root, { recursive: true })).filter((path) => /\.(rs|ts|svelte)$/.test(path)).sort();
  return (await Promise.all(files.map((path) => readFile(join(root, path), "utf8")))).join("\n");
}

export function containsKey(value: unknown, key: string): boolean {
  if (Array.isArray(value)) return value.some((entry) => containsKey(entry, key));
  if (value !== null && typeof value === "object") return Object.entries(value).some(([candidate, entry]) => candidate === key || containsKey(entry, key));
  return false;
}
