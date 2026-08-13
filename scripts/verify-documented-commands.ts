/**
 * Every `effigy <task>` named in a README must resolve.
 *
 * Nineteen dead commands accumulated in `examples/` READMEs before anyone
 * noticed, and what surfaced it was writing a twentieth proof and copying the
 * convention — which meant copying a command that had not existed for weeks.
 *
 * Documentation drift is usually cosmetic. This kind is not: a packaged proof
 * is only worth having if someone can run it, and the instruction for running
 * it is the whole interface. A README that names a task nobody can invoke
 * describes a capability the repository does not have.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

/**
 * Task names `effigy` will accept.
 *
 * Two shapes in `effigy.toml`: quoted keys under `[tasks]`, and bare keys whose
 * value is an array, which are the aggregates like `qa`. Reading only the first
 * shape reports `qa` as missing, which is how the first measurement of this
 * drift came out one too high.
 */
function definedTasks(): Set<string> {
  const manifest = readFileSync(join(repoRoot, "effigy.toml"), "utf8");
  const names = new Set<string>();
  for (const [, name] of manifest.matchAll(/^"([a-z0-9:_.-]+)"\s*=/gm)) names.add(name!);
  for (const [, name] of manifest.matchAll(/^([a-z0-9:_.-]+)\s*=\s*\[/gm)) names.add(name!);
  return names;
}

function markdownFiles(directory: string, found: string[] = []): string[] {
  for (const entry of readdirSync(directory)) {
    if (entry === "node_modules" || entry === "target" || entry === "gen") continue;
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) markdownFiles(path, found);
    else if (entry.endsWith(".md")) found.push(path);
  }
  return found;
}

const tasks = definedTasks();
const dead: string[] = [];
let checked = 0;

for (const file of markdownFiles(join(repoRoot, "examples"))) {
  const text = readFileSync(file, "utf8");
  for (const [, name] of text.matchAll(/\beffigy ([a-z0-9:_-]+)/g)) {
    checked += 1;
    if (!tasks.has(name!)) dead.push(`${relative(repoRoot, file)}: effigy ${name}`);
  }
}

if (dead.length > 0) {
  console.error(
    `${dead.length} of ${checked} documented effigy commands do not exist:\n` +
      dead.map((entry) => `  ${entry}`).join("\n") +
      "\n\nEither define the task, or document the command that does the work now.",
  );
  process.exit(1);
}

console.log(
  JSON.stringify({
    schema: "longhorn.documented-commands.v1",
    outcome: "pass",
    commandsChecked: checked,
    tasksDefined: tasks.size,
  }),
);
