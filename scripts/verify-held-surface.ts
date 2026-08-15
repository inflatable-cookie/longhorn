import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const register = await readFile(join(repoRoot, "docs/reference/held-surface.md"), "utf8");
const api = await readFile(join(repoRoot, "docs/reference/api-surface.md"), "utf8");
const selection = await readFile(join(repoRoot, "docs/guides/package-selection.md"), "utf8");

const statuses = new Set(["held-for-consumer", "planning-evidence"]);

const rows = [...register.matchAll(/^\| ([^|]+) \| (held-for-consumer|planning-evidence) \| ([^|]+) \| ([^|]+) \|$/gm)];
if (rows.length === 0) throw new Error("held-surface register has no parseable rows");

for (const row of rows) {
  const [surface, status, awaits, trigger] = row.slice(1);
  if (!statuses.has(status!)) throw new Error(`held surface has invalid status: ${surface}`);
  if (awaits!.trim() === "" || trigger!.trim() === "") {
    throw new Error(`held surface must name what it awaits and its trigger: ${surface}`);
  }
}

const heldCrates = [
  "longhorn-config-age",
  "longhorn-bridge",
  "longhorn-history-tree",
];
const heldPackages = ["@inflatable-cookie/longhorn/history-tree"];

for (const crate of heldCrates) {
  if (!api.includes(`\`${crate}\``)) throw new Error(`held crate missing from api-surface: ${crate}`);
}
for (const packageName of heldPackages) {
  if (!api.includes(`\`${packageName}\``)) {
    throw new Error(`held package missing from api-surface: ${packageName}`);
  }
}

// Freshness: a row that awaits closed work is stale by definition. Resolve
// the milestone (g02.NNN) and card (NNN) references in each row's Awaits and
// Trigger columns against the roadmap files' status lines.
const { readdir } = await import("node:fs/promises");

async function statusIndex(directory: string): Promise<Map<string, string>> {
  const index = new Map<string, string>();
  for (const file of await readdir(join(repoRoot, directory))) {
    if (!file.endsWith(".md")) continue;
    const content = await readFile(join(repoRoot, directory, file), "utf8");
    index.set(file, content);
  }
  return index;
}

const milestones = await statusIndex("docs/roadmaps/g02");
const cards = await statusIndex("docs/roadmaps/g02/batch-cards");

function isClosed(reference: string, files: Map<string, string>, prefix: string): boolean {
  for (const [file, content] of files) {
    if (!file.startsWith(prefix)) continue;
    return /^Status: (complete|superseded)/m.test(content);
  }
  return false;
}

for (const row of rows) {
  const [surface, , awaits, trigger] = row.slice(1);
  for (const match of `${awaits} ${trigger}`.matchAll(/g02\.(\d{3})/g)) {
    if (isClosed(match[0], milestones, `${match[1]}-`)) {
      throw new Error(`held surface "${surface!.trim()}" awaits ${match[0]}, which is closed`);
    }
  }
  for (const match of `${awaits} ${trigger}`.matchAll(/card (?:\()?(\d{3})/gi)) {
    if (isClosed(match[1]!, cards, `${match[1]}-`)) {
      throw new Error(`held surface "${surface!.trim()}" awaits card ${match[1]}, which is closed`);
    }
  }
}

const selectionSection = selection.split("## Held Surface", 1)[0]!;
const selectableBlock = selectionSection.split("## Add One System At A Time")[1] ?? "";
for (const crate of heldCrates) {
  if (crate === "longhorn-bridge") continue;
  if (selectableBlock.includes(crate)) {
    throw new Error(`held crate is presented as selectable in package-selection: ${crate}`);
  }
}
if (selectableBlock.includes("supervision")) {
  throw new Error("held bridge supervision is presented as selectable in package-selection");
}

const heldSection = selection.split("## Held Surface")[1] ?? "";
if (!heldSection.includes("../reference/held-surface.md")) {
  throw new Error("package-selection held surface section does not link the register");
}
if (!heldSection.includes("planning-evidence")) {
  throw new Error("package-selection held surface section must state the planning-evidence status");
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.held-surface-proof.v1",
      outcome: "pass",
      rows: rows.length,
      heldCrates,
      heldPackages,
      selectableSurface: "excluded",
    },
    null,
    2,
  ),
);
