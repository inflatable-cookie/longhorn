import { requireValue } from "./proof-support.ts";
import { readdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const register = await readFile(join(repoRoot, "docs/reference/held-surface.md"), "utf8");
const api = await readFile(join(repoRoot, "docs/reference/api-surface.md"), "utf8");
const selection = await readFile(join(repoRoot, "docs/guides/package-selection.md"), "utf8");

const statuses = new Set(["held-for-consumer", "planning-evidence"]);

// --- Held register ------------------------------------------------------

const rows = [
  ...register.matchAll(
    /^\| ([^|]+) \| (held-for-consumer|planning-evidence) \| ([^|]+) \| ([^|]+) \|$/gm,
  ),
];
if (rows.length === 0) {
  throw new Error("held-surface register has no parseable held rows");
}

for (const row of rows) {
  const [surface, status, awaits, trigger] = row.slice(1);
  if (!statuses.has(requireValue(status, "status"))) {
    throw new Error(`held surface has invalid status: ${surface}`);
  }
  if (
    requireValue(awaits, "awaits").trim() === "" ||
    requireValue(trigger, "trigger").trim() === ""
  ) {
    throw new Error(`held surface must name what it awaits and its trigger: ${surface}`);
  }
}

// --- Assessed and retained table ---------------------------------------

// Owner names come from the generated API surface, so a row cannot point at a
// crate or package that no longer exists. Example owners are admitted only
// under `examples/`, where a contract requires evidence rather than API.
const crateRoots = new Map<string, string>();
for (const match of api.matchAll(
  /^\| `([a-z0-9-]+)` \| library \|.*\| \[source\]\(([^)]+)\) \|$/gm,
)) {
  const [, crate, source] = match;
  const path = requireValue(source, "source")
    .replace(/^\.\.\/\.\.\//, "")
    .split("/")
    .slice(0, 2)
    .join("/");
  crateRoots.set(requireValue(crate, "crate"), path);
}

const packageRoots = new Map<string, string>();
for (const match of api.matchAll(/^\| `(@[^`]+)` \|.*\| \[README\]\(([^)]+)\) \|$/gm)) {
  const [, name, reference] = match;
  const path = requireValue(reference, "reference")
    .replace(/^\.\.\/\.\.\//, "")
    .split("/")
    .slice(0, 2)
    .join("/");
  packageRoots.set(requireValue(name, "name"), path);
}

if (crateRoots.size === 0) {
  throw new Error("api-surface has no parseable crate table");
}
if (packageRoots.size === 0) {
  throw new Error("api-surface has no parseable package table");
}

if (!/^\| Item \| Crate or package \| Why retained \| Evidence \| Reviewed \|$/m.test(register)) {
  throw new Error("held-surface register is missing the Assessed And Retained table header");
}

const retained = [
  ...register.matchAll(/^\| ([^|]+) \| ([^|]+) \| ([^|]+) \| ([^|]+) \| (\d{4}-\d{2}-\d{2}) \|$/gm),
];
if (retained.length === 0) {
  throw new Error("held-surface register has no parseable retained rows");
}

const ignoredDirectories = new Set([".git", "node_modules", "target", "dist", ".svelte-kit"]);

// The public surface lives in `src`, so an integration test under `tests/`
// cannot keep a row alive once the item is gone; unit-test modules inside
// `src` still count as text. A package's manifest counts because export
// subpaths are surface.
async function treeText(roots: ReadonlyArray<string>): Promise<string> {
  const chunks: Array<string> = [];
  async function walk(root: string, relative: string): Promise<void> {
    const entries = await readdir(join(repoRoot, root, relative), { withFileTypes: true });
    for (const entry of entries) {
      const child = relative === "" ? entry.name : join(relative, entry.name);
      if (entry.isDirectory()) {
        if (ignoredDirectories.has(entry.name)) {
          continue;
        }
        await walk(root, child);
        continue;
      }
      if (!entry.isFile()) {
        continue;
      }
      chunks.push(child, await readFile(join(repoRoot, root, child), "utf8"));
    }
  }
  for (const root of roots) {
    chunks.push(root);
    if (root.endsWith(".json")) {
      chunks.push(await readFile(join(repoRoot, root), "utf8"));
      continue;
    }
    await walk(root, "");
  }
  return chunks.join("\n");
}

// `TransferAbort.retryable` and `DocumentFormat::Toml` name one declaration in
// a larger type, and `native_platform_directory_fact*` names a family, so the
// searchable needle is the last path segment without a trailing glob or slash.
function needleFor(token: string): string {
  const stripped = token
    .trim()
    .replace(/^\.\//, "")
    .replace(/[*/]+$/, "");
  // `./package.json` is a file name, not a member access.
  if (/^[\w-]+\.(?:json|ts|js|md|svelte)$/.test(stripped)) {
    return stripped;
  }
  const member = stripped.includes("::")
    ? requireValue(stripped.split("::").at(-1), 'stripped.split("::").at(-1)')
    : stripped.includes(".")
      ? requireValue(stripped.split(".").at(-1), 'stripped.split(".").at(-1)')
      : stripped;
  return requireValue(member.split("/").at(-1), 'member .split("/") .at(-1)').replace(/[*/]+$/, "");
}

const trees = new Map<string, string>();

function surfacePaths(ownerName: string, root: string): Array<string> {
  if (ownerName.startsWith("examples/")) {
    return [root];
  }
  if (root.startsWith("packages/")) {
    return [`${root}/src`, `${root}/package.json`];
  }
  return [`${root}/src`];
}

for (const row of retained) {
  const itemCell = requireValue(row[1], "held item");
  const owner = requireValue(row[2], "held item owner");
  const why = requireValue(row[3], "retention reason");
  const evidence = requireValue(row[4], "retention evidence");
  const reviewed = requireValue(row[5], "review date");
  const items = [...itemCell.matchAll(/`([^`]+)`/g)].map((match) =>
    requireValue(match[1], "match[1]").trim(),
  );
  if (items.length === 0) {
    throw new Error(`retained row must name its items in backticks: ${itemCell.trim()}`);
  }
  if (why.trim() === "") {
    throw new Error(`retained row must say why it is kept: ${itemCell.trim()}`);
  }
  // A retained decision is durable only while its evidence still says the same
  // thing. A task or card ID closes, so it can never be that evidence.
  if (/\b(?:g0\d\.\d{3}|(?:card|task|longhorn)\s*#?\s*\d{2,4})\b/i.test(`${why} ${evidence}`)) {
    throw new Error(
      `retained row "${itemCell.trim()}" names a task or card; cite the contract, guide, reference or consumer instead`,
    );
  }
  if (!/\bcontract \d{3}\b|\.md\b/.test(evidence)) {
    throw new Error(
      `retained row "${itemCell.trim()}" must cite a contract section, guide or reference doc`,
    );
  }
  if (!/^\d{4}-\d{2}-\d{2}$/.test(reviewed)) {
    throw new Error(`retained row "${itemCell.trim()}" has an invalid Reviewed date: ${reviewed}`);
  }

  const ownerName = owner.trim().replaceAll(/^`|`$/g, "");
  let root = crateRoots.get(ownerName) ?? packageRoots.get(ownerName);
  if (root === undefined) {
    if (!ownerName.startsWith("examples/")) {
      throw new Error(
        `retained row "${itemCell.trim()}" names a crate or package missing from docs/reference/api-surface.md: ${ownerName}`,
      );
    }
    root = ownerName;
    try {
      await readdir(join(repoRoot, root));
    } catch {
      throw new Error(
        `retained row "${itemCell.trim()}" names a missing example directory: ${ownerName}`,
      );
    }
  }

  const paths = surfacePaths(ownerName, root);
  let tree = trees.get(paths.join("|"));
  if (tree === undefined) {
    try {
      tree = await treeText(paths);
    } catch {
      throw new Error(
        `retained row "${itemCell.trim()}" names an owner with no readable source: ${ownerName} (${root})`,
      );
    }
    trees.set(paths.join("|"), tree);
  }

  for (const item of items) {
    const needle = needleFor(item);
    if (needle === "") {
      throw new Error(`retained row has an empty item: ${itemCell.trim()}`);
    }
    if (!tree.includes(needle)) {
      throw new Error(
        `retained item \`${item}\` (searching "${needle}") is not in ${ownerName}'s source (${paths.join(", ")}); ` +
          "if it was renamed or removed, update or delete its row in the same PR",
      );
    }
  }
}

// --- Freshness ----------------------------------------------------------

// A held row must name the consumer need or product choice that moves it,
// never a task or card ID. Task status lives in Queue, not in this repository,
// so a task reference here could never be checked for closure.
for (const row of rows) {
  const [surface, , awaits, trigger] = row.slice(1);
  if (/\bg0\d\.\d{3}\b|\bcard \(?\d{3}/i.test(`${awaits} ${trigger}`)) {
    throw new Error(
      `held surface "${requireValue(surface, "surface").trim()}" names a task or card; name the consumer trigger instead`,
    );
  }
}

// --- Held surface is not selectable -------------------------------------

const heldCrates = ["longhorn-config-age", "longhorn-bridge"];
const heldPackages: Array<string> = [];

for (const crate of heldCrates) {
  if (!api.includes(`\`${crate}\``)) {
    throw new Error(`held crate missing from api-surface: ${crate}`);
  }
}
for (const packageName of heldPackages) {
  if (!api.includes(`\`${packageName}\``)) {
    throw new Error(`held package missing from api-surface: ${packageName}`);
  }
}

const selectionSection = requireValue(
  selection.split("## Held Surface", 1)[0],
  'selection.split("## Held Surface", 1)[0]',
);
const selectableBlock = selectionSection.split("## Add One System At A Time")[1] ?? "";
for (const crate of heldCrates) {
  if (crate === "longhorn-bridge") {
    continue;
  }
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
      retainedRows: retained.length,
      heldCrates,
      heldPackages,
      selectableSurface: "excluded",
    },
    null,
    2,
  ),
);
