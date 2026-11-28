import { requireRecord } from "./proof-json.ts";
import { requireValue, run } from "./proof-support.ts";
import { access, readdir, readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const guidePaths = [
  "docs/guides/README.md",
  "docs/guides/getting-started.md",
  "docs/guides/package-selection.md",
  "docs/guides/storage-configuration-backup.md",
  "docs/guides/system-composition.md",
  "docs/guides/migration-and-rollback.md",
  "docs/guides/compatibility-and-upgrades.md",
];
const checkedPaths = [
  ...guidePaths,
  "docs/reference/README.md",
  "docs/reference/api-surface.md",
  "docs/README.md",
  "README.md",
  "examples/greenfield-compositions/README.md",
];

async function main(): Promise<void> {
  await run(["bun", "scripts/generate-api-reference.ts"], repoRoot);
  const documents = new Map<string, string>();
  for (const path of checkedPaths) {
    documents.set(path, await readFile(join(repoRoot, path), "utf8"));
  }
  await verifyLocalLinks(documents);

  const guides = guidePaths
    .map((path) => requireValue(documents.get(path), "documents.get(path)"))
    .join("\n");
  requireAll(guides, [
    "canonical application id",
    "display name",
    "stable storage name",
    "platform-native-v1",
    "unified-app-root-v1",
    "shared-product-root-v1",
    "portable-v1",
    "Application Support/<leaf>/config",
    String.raw`%LOCALAPPDATA%\<leaf>\config`,
    "$XDG_CONFIG_HOME/<leaf>",
    ".longhorn-backup",
    ".longhorn-backup.age",
    "recovery required",
    "linear structure",
    "product snapshot",
    "project versions",
    "Fork-tree",
    "silent fallback",
    "dual-write",
    "receipt-bound cleanup",
    "Windows/Linux unproved",
    "Windows/Linux unsupported",
  ]);

  const api = requireValue(
    documents.get("docs/reference/api-surface.md"),
    'documents.get("docs/reference/api-surface.md")',
  );
  const rustCount = [...api.matchAll(/^\| `longhorn-[^`]+` \|/gm)].length;
  const longhornPackageNames = [
    ...api.matchAll(/^\| `(@inflatable-cookie\/longhorn[^`]*)` \|/gm),
  ].map((match) => requireValue(match[1], "API package name"));
  const typescriptCount = longhornPackageNames.length;
  const crateDirectories = (
    await readdir(join(repoRoot, "crates"), { withFileTypes: true })
  ).filter((entry) => entry.isDirectory()).length;
  const packageDirectories = (
    await readdir(join(repoRoot, "packages"), { withFileTypes: true })
  ).filter((entry) => entry.isDirectory()).length;
  if (rustCount !== crateDirectories || typescriptCount !== packageDirectories) {
    throw new Error(
      `API inventory does not list the workspace: api-surface.md has ${rustCount} crates and ${typescriptCount} packages, ` +
        `the workspace has ${crateDirectories} and ${packageDirectories}. Run \`effigy generate:api-reference\`.`,
    );
  }
  const publicPackages = new Set([
    ...longhornPackageNames,
    "@inflatable-cookie/poodle-core",
    "@inflatable-cookie/poodle-svelte",
  ]);
  for (const [path, content] of documents) {
    verifySnippetBoundaries({ path, content, publicPackages });
  }
  verifyGettingStarted(
    requireValue(documents.get("docs/guides/getting-started.md"), "getting-started guide"),
  );

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.guide-corpus.v2",
        outcome: "pass",
        guides: guidePaths.length,
        checkedDocuments: checkedPaths.length,
        localLinks: "pass",
        snippets: "public-or-artifact-paths",
        npmInstall: "published-packages",
        rustInstall: "git-tag",
        rustPackages: rustCount,
        typescriptPackages: typescriptCount,
      },
      null,
      2,
    ),
  );
}

async function verifyLocalLinks(documents: Map<string, string>): Promise<void> {
  for (const [path, content] of documents) {
    for (const match of content.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)) {
      const target = requireValue(match[1], "match[1]");
      if (/^(?:https?:|mailto:|#)/.test(target)) {
        continue;
      }
      const filesystemTarget = requireValue(target.split("#", 1)[0], 'target.split("#", 1)[0]');
      try {
        await access(resolve(repoRoot, dirname(path), filesystemTarget));
      } catch {
        throw new Error(`${path} has missing local link ${target}`);
      }
    }
  }
}

function codeBlocks(content: string, language = String.raw`[^\n]*`): Array<string> {
  const fence = new RegExp("```" + language + "\\n([\\s\\S]*?)```", "g");
  return [...content.matchAll(fence)].map((match) => requireValue(match[1], "code block"));
}

export function verifySnippetBoundaries({
  path,
  content,
  publicPackages,
}: {
  path: string;
  content: string;
  publicPackages: ReadonlySet<string>;
}): void {
  const forbidden = [
    /cargo\s+add/,
    /@inflatable-cookie\/(?:longhorn|poodle)[^\s`"']*\/src\//,
    /@inflatable-cookie\/poodle-[^\s`"']*(?:internal|private)/,
    /(?:\.\.\/)+packages\//,
    /workspace:\*/,
  ];
  for (const block of codeBlocks(content)) {
    for (const match of block
      .replaceAll(/\\\r?\n/g, " ")
      .matchAll(/(?:^|\s)(?:npm|pnpm|yarn|bun)\s+(?:add|install)\s+([^;&|\n]+)/g)) {
      const args = requireValue(match[1], "registry install arguments");
      for (const arg of args.trim().split(/\s+/)) {
        const spec =
          arg
            .replaceAll(/^['"]|['"]$/g, "")
            .split("@npm:")
            .at(-1) ?? "";
        const name =
          /^(?:@inflatable-cookie\/(?:longhorn|poodle)[^@]*|@(?:longhorn|poodle)\/[^@]+|longhorn-[^@]+)(?=@|$)/.exec(
            spec,
          )?.[0];
        const localTarball = /^longhorn-[\w.-]+\.tgz$/.test(spec);
        if (name !== undefined && !localTarball && !publicPackages.has(name)) {
          throw new Error(`${path} installs unpublished package or subpath ${spec} from npm`);
        }
      }
    }
    for (const pattern of forbidden) {
      if (pattern.test(block)) {
        throw new Error(`${path} contains non-public snippet ${pattern}`);
      }
    }
  }
}

export function verifyGettingStarted(content: string): void {
  const dependencies = new Set<string>();
  const rendererPins = new Map(
    [
      ...codeBlocks(content, "sh")
        .join("\n")
        .matchAll(/(@inflatable-cookie\/longhorn(?:-tauri|-poodle-svelte)?)@([^\s]+)/g),
    ].map((match) => [
      requireValue(match[1], "renderer package"),
      requireValue(match[2], "renderer version"),
    ]),
  );
  const version = rendererPins.get("@inflatable-cookie/longhorn");
  if (
    version === undefined ||
    !/^\d+\.\d+\.\d+$/.test(version) ||
    rendererPins.get("@inflatable-cookie/longhorn-tauri") !== version ||
    rendererPins.get("@inflatable-cookie/longhorn-poodle-svelte") !== version
  ) {
    throw new Error("getting-started renderer installs must share one exact Longhorn version");
  }
  for (const block of codeBlocks(content, "toml")) {
    const manifest = requireRecord(Bun.TOML.parse(block), "getting-started Cargo example");
    for (const [name, spec] of Object.entries(
      requireRecord(manifest.dependencies, "dependencies"),
    )) {
      if (!name.startsWith("longhorn-")) {
        continue;
      }
      const source = requireRecord(spec, `${name} Git dependency`);
      if (
        source.git !== "ssh://git@github.com/inflatable-cookie/longhorn.git" ||
        source.tag !== `v${version}`
      ) {
        throw new Error(`${name} must use the Longhorn Git tag v${version}`);
      }
      dependencies.add(name);
    }
  }
  if (dependencies.size === 0) {
    throw new Error("getting-started Cargo dependencies are missing");
  }
  const commands = [
    ...codeBlocks(content, "rust")
      .join("\n")
      .matchAll(/\b(longhorn_tauri_[a-z_]+)::longhorn_\w+/g),
  ];
  if (commands.length === 0) {
    throw new Error("getting-started Tauri handler example is missing");
  }
  for (const match of commands) {
    const crate = requireValue(match[1], "Tauri command owner").replaceAll("_", "-");
    if (!dependencies.has(crate)) {
      throw new Error(`getting-started Tauri handler requires missing dependency ${crate}`);
    }
  }
}

function requireAll(content: string, required: Array<string>): void {
  for (const value of required) {
    if (!content.includes(value)) {
      throw new Error(`guide corpus missing required claim: ${value}`);
    }
  }
}

if (import.meta.main) {
  await main();
}
