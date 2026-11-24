import { expect, test } from "bun:test";
import { readdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

// AGENTS.md names the `doctor` health list and the self-re-executing test
// crates. Those lists drift when a task or a test changes, so they are checked
// against their sources here instead of by reading.
const repoRoot = resolve(import.meta.dir, "..");

function section(markdown: string, start: string, end: string): string {
  const from = markdown.indexOf(start);
  const to = markdown.indexOf(end, from);
  if (from < 0 || to < 0) {
    throw new Error(`AGENTS.md section from "${start}" to "${end}" not found`);
  }
  return markdown.slice(from, to);
}

function backticked(text: string, pattern: RegExp): Array<string> {
  return [...text.matchAll(/`([^`]+)`/g)].flatMap((match) => {
    const name = match[1] ?? "";
    return pattern.test(name) ? [name] : [];
  });
}

test("the doctor health list in AGENTS.md names every health task", async () => {
  const [agents, effigy] = await Promise.all([
    readFile(join(repoRoot, "AGENTS.md"), "utf8"),
    readFile(join(repoRoot, "effigy.toml"), "utf8"),
  ]);
  const healthBlock = /^health = \[([\s\S]*?)^\]/m.exec(effigy)?.[1];
  if (healthBlock === undefined) {
    throw new Error("effigy.toml has no health list");
  }
  const tasks = [...healthBlock.matchAll(/task = "([^"]+)"/g)].map((match) => match[1] ?? "");

  const doctor = section(agents, "`doctor` is cheap orientation", "built-in scans");
  const named = backticked(doctor, /^[a-z][a-z-]*:[a-z-]+$/);

  expect([...named].sort()).toEqual([...tasks].sort());
});

test("the self-re-executing test crates in AGENTS.md are every crate that re-runs its own binary", async () => {
  const agents = await readFile(join(repoRoot, "AGENTS.md"), "utf8");
  const claim = section(agents, "Integration tests that re-exec themselves", "need no change");
  const named = backticked(claim, /^longhorn-[a-z-]+$/);

  const cratesRoot = join(repoRoot, "crates");
  const files = (await readdir(cratesRoot, { recursive: true })).filter((path) =>
    path.endsWith(".rs"),
  );
  const reexec = new Set<string>();
  for (const path of files) {
    const source = await readFile(join(cratesRoot, path), "utf8");
    if (/current_exe\(\)/.test(source)) {
      reexec.add(path.split("/")[0] ?? "");
    }
  }

  expect([...new Set(named)].sort()).toEqual([...reexec].sort());
});
