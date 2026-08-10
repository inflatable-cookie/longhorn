import { access, readdir, readFile } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";

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

await run(["bun", "scripts/generate-api-reference-card126.ts"]);
const documents = new Map<string, string>();
for (const path of checkedPaths) documents.set(path, await readFile(join(repoRoot, path), "utf8"));
await verifyLocalLinks(documents);
verifySnippetBoundaries(documents);

const guides = guidePaths.map((path) => documents.get(path)!).join("\n");
requireAll(guides, [
  "canonical application id",
  "display name",
  "stable storage name",
  "platform-native-v1",
  "unified-app-root-v1",
  "shared-product-root-v1",
  "portable-v1",
  "Application Support/<leaf>/config",
  "%LOCALAPPDATA%\\<leaf>\\config",
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
  "no npm/crates.io install",
]);

const api = documents.get("docs/reference/api-surface.md")!;
// Derived from the workspace rather than hardcoded. A literal count means
// every new crate reddens this gate with a message that names a generator
// selector rather than "you added a crate" -- which is exactly what happened
// when longhorn-gpui-windowing landed. The claim worth keeping is that the
// generated inventory lists everything, not that the total equals a number
// someone wrote down.
const rustCount = [...api.matchAll(/^\| `longhorn-[^`]+` \|/gm)].length;
const typescriptCount = [
  ...api.matchAll(/^\| `@inflatable-cookie\/longhorn[^`]*` \|/gm),
].length;
const crateDirectories = (await readdir(join(repoRoot, "crates"), { withFileTypes: true }))
  .filter((entry) => entry.isDirectory())
  .length;
const packageDirectories = (await readdir(join(repoRoot, "packages"), { withFileTypes: true }))
  .filter((entry) => entry.isDirectory())
  .length;
if (rustCount !== crateDirectories || typescriptCount !== packageDirectories) {
  throw new Error(
    `API inventory does not list the workspace: api-surface.md has ${rustCount} crates and ${typescriptCount} packages, ` +
      `the workspace has ${crateDirectories} and ${packageDirectories}. Run \`effigy generate:api-reference\`.`,
  );
}
// Card 166 made the three TypeScript packages publishable, so the posture the
// reference must state is no longer "private everywhere". The Rust half is
// unchanged: `publish = false`, nothing on crates.io.
if (!api.includes("not on crates.io") || !api.includes("publishConfig.access: public") || !api.includes("publish = false")) {
  throw new Error("API reference publication posture missing");
}

console.log(JSON.stringify({
  schema: "longhorn.card126-guide-proof.v1",
  outcome: "pass",
  guides: guidePaths.length,
  checkedDocuments: checkedPaths.length,
  localLinks: "pass",
  snippets: "public-or-artifact-paths",
  registryInstall: "forbidden",
  rustPackages: rustCount,
  typescriptPackages: typescriptCount,
  packageManagerPublication: false,
}, null, 2));

async function verifyLocalLinks(documents: Map<string, string>): Promise<void> {
  for (const [path, content] of documents) {
    for (const match of content.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)) {
      const target = match[1]!;
      if (/^(?:https?:|mailto:|#)/.test(target)) continue;
      const filesystemTarget = target.split("#", 1)[0]!;
      try {
        await access(resolve(repoRoot, dirname(path), filesystemTarget));
      } catch {
        throw new Error(`${path} has missing local link ${target}`);
      }
    }
  }
}

function verifySnippetBoundaries(documents: Map<string, string>): void {
  const forbidden = [
    /cargo\s+add/,
    /@inflatable-cookie\/poodle-[^\s`"']*(?:internal|private)/,
    /@longhorn\/[^\s`"']+\/src\//,
    /(?:\.\.\/)+packages\//,
    /workspace:\*/,
  ];
  for (const [path, content] of documents) {
    const blocks = [...content.matchAll(/```[^\n]*\n([\s\S]*?)```/g)].map((match) => match[1]!);
    for (const block of blocks) {
      for (const line of block.split("\n")) {
        const target = registryInstallTarget(line);
        if (target !== null) {
          throw new Error(
            `${path} contains a registry-style install of private package ${target}; ` +
              "use artifact paths (./tarballs) or third-party registries only",
          );
        }
      }
      for (const pattern of forbidden) {
        if (pattern.test(block)) throw new Error(`${path} contains non-public snippet ${pattern}`);
      }
    }
  }
}

// Guides may show installs from produced private tarballs (./artifacts/...) and
// from public third-party registries (svelte, @tauri-apps/api). They must never
// imply that a Longhorn or Poodle package is installable from a registry.
function registryInstallTarget(line: string): string | null {
  for (const match of line.matchAll(/(?:^|\s)(?:npm|pnpm|yarn|bun)\s+(?:add|install)\s+/g)) {
    const rest = line.slice(match.index! + match[0].length).split(/[;&|]/, 1)[0]!;
    for (const token of rest.trim().split(/\s+/)) {
      if (token === "" || token.startsWith("-")) continue;
      if (/^@(?:longhorn|poodle)\/[\w.-]+$/.test(token)) return token;
      if (/^longhorn-[\w.-]+$/.test(token) && !token.endsWith(".tgz")) return token;
      break;
    }
  }
  return null;
}

function requireAll(content: string, required: string[]): void {
  for (const value of required) {
    if (!content.includes(value)) throw new Error(`guide corpus missing required claim: ${value}`);
  }
}

async function run(command: string[]): Promise<string> {
  const process = Bun.spawn(command, { cwd: repoRoot, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
    process.exited,
  ]);
  if (exitCode !== 0) throw new Error(`${command.join(" ")} failed\n${stdout}\n${stderr}`);
  return stdout;
}
