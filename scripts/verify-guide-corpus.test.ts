import { expect, test } from "bun:test";
import { readdir, readFile } from "node:fs/promises";
import { requireRecord } from "./proof-json.ts";
import { verifyGettingStarted, verifySnippetBoundaries } from "./verify-guide-corpus.ts";

const publicPackages = new Set([
  "@inflatable-cookie/longhorn",
  "@inflatable-cookie/longhorn-tauri",
  "@inflatable-cookie/longhorn-poodle-svelte",
  "@inflatable-cookie/poodle-core",
  "@inflatable-cookie/poodle-svelte",
]);

function verifyShell(command: string): void {
  verifySnippetBoundaries({
    path: "example.md",
    content: `\`\`\`sh\n${command}\n\`\`\``,
    publicPackages,
  });
}

test("registry installs can mix published Longhorn, Poodle and third-party packages", () => {
  verifyShell(
    "bun add --exact svelte @inflatable-cookie/longhorn@0.6.0 @inflatable-cookie/poodle-svelte",
  );
  verifyShell("npm install ./artifacts/longhorn-config.tgz");
});

test("private packages and export subpaths are rejected even after a valid package", () => {
  for (const spec of [
    "@inflatable-cookie/longhorn-config",
    "@inflatable-cookie/poodle-preview@1.2.3",
    "@inflatable-cookie/longhorn/config",
    "longhorn-config@0.6.0",
    "@inflatable-cookie/longhorn-config.tgz",
  ]) {
    expect(() => verifyShell(`npm install svelte \\\n  "${spec}"`)).toThrow(spec);
  }
  expect(() => verifyShell("npm install config@npm:@inflatable-cookie/longhorn-config")).toThrow(
    "@inflatable-cookie/longhorn-config",
  );
});

test("guides use the Rust Git tag rather than Cargo registry installation", () => {
  expect(() => verifyShell("cargo add longhorn-config")).toThrow("non-public snippet");
});

const guide = [
  "```toml",
  "[dependencies]",
  'longhorn-tauri-settings = { git = "ssh://git@github.com/inflatable-cookie/longhorn.git", tag = "v0.6.0" }',
  "```",
  "```rust",
  "longhorn_tauri_settings::longhorn_settings_registry,",
  "```",
  "```sh",
  "bun add --exact @inflatable-cookie/longhorn@0.6.0 @inflatable-cookie/longhorn-tauri@0.6.0",
  "bun add --exact @inflatable-cookie/longhorn-poodle-svelte@0.6.0",
  "```",
].join("\n");

test("the renderer and host recipes use one coordinated release", () => {
  verifyGettingStarted(guide);
  expect(() => verifyGettingStarted(guide.replace('tag = "v0.6.0"', 'tag = "v0.1.0"'))).toThrow(
    "must use the Longhorn Git tag v0.6.0",
  );
  expect(() => verifyGettingStarted(guide.replace(/\{ git = .* \}/, '"0.1.0"'))).toThrow(
    "Git dependency must be an object",
  );
  expect(() =>
    verifyGettingStarted(guide.replace("longhorn-tauri@0.6.0", "longhorn-tauri@0.1.0")),
  ).toThrow("one exact Longhorn version");
});

test("command registration must use a crate in the dependency recipe", () => {
  expect(() =>
    verifyGettingStarted(
      guide.replace(
        "longhorn_tauri_settings::longhorn_settings_registry,",
        "longhorn_tauri_notifications::longhorn_notifications_snapshot,",
      ),
    ),
  ).toThrow("requires missing dependency longhorn-tauri-notifications");
});

async function readRepoText(path: string): Promise<string> {
  return readFile(new URL(`../${path}`, import.meta.url), "utf8");
}

test("geometry docs name the window-content unit exported by the core", async () => {
  const source = await readRepoText("crates/longhorn-core/src/client_geometry.rs");
  const unit = /^pub struct (Client\w+Px)\(f64\);$/m.exec(source)?.[1];
  expect(unit).toBeDefined();
  for (const path of [
    "docs/knowledge/contracts/009-display-identity-coordinates-and-window-planning.md",
    "docs/knowledge/contracts/017-native-content-island-coordination.md",
    "docs/knowledge/architecture/system-architecture.md",
  ]) {
    expect(await readRepoText(path)).toContain(`\`${unit}\``);
  }
});

test("the Tauri update proof links its workspace manifest and the separate adapter", async () => {
  const root = "examples/tauri-update-proof";
  const manifestPath = "src-tauri/Cargo.toml";
  const [guideText, workspaceText, manifestText] = await Promise.all([
    readRepoText(`${root}/README.md`),
    readRepoText("Cargo.toml"),
    readRepoText(`${root}/${manifestPath}`),
  ]);
  const workspaceManifest = requireRecord(Bun.TOML.parse(workspaceText), "Cargo manifest");
  const workspace = requireRecord(workspaceManifest.workspace, "workspace");
  expect(workspace.members).toContain(`${root}/src-tauri`);
  expect(guideText).toContain(`(${manifestPath})`);
  expect(guideText).not.toMatch(/not a workspace member|scaffold, not yet a proof/i);
  const proofManifest = requireRecord(Bun.TOML.parse(manifestText), "proof manifest");
  const dependencies = requireRecord(proofManifest.dependencies, "proof dependencies");
  expect(dependencies).toHaveProperty("longhorn-update");
  expect(dependencies).not.toHaveProperty("longhorn-tauri-update");
  expect(guideText).toContain("(../../crates/longhorn-tauri-update/src/lib.rs)");
});

test("the installation recipe uses the latest verified publication receipt", async () => {
  const [changelog, gettingStarted] = await Promise.all([
    readRepoText("CHANGELOG.md"),
    readRepoText("docs/guides/getting-started.md"),
  ]);
  // Release preparation adds a version heading before the packages ship.
  const publishedRelease = changelog
    .split(/^## /m)
    .find((section) => /^- \*\*Publication evidence\.\*\*/m.test(section));
  const version = /^\[(\d+\.\d+\.\d+)\]/.exec(publishedRelease ?? "")?.[1];
  expect(version).toBeDefined();
  const pins = [
    ...gettingStarted.matchAll(
      /@inflatable-cookie\/longhorn(?:-tauri|-poodle-svelte)?@(\d+\.\d+\.\d+)/g,
    ),
  ];
  expect(pins.length).toBeGreaterThanOrEqual(3);
  expect([...new Set(pins.map((match) => match[1]))]).toEqual([version]);
  expect(gettingStarted).toContain(`Longhorn ${version} release`);
});

const publicationDocs = [
  "docs/guides/README.md",
  "docs/guides/getting-started.md",
  "docs/guides/package-selection.md",
  "docs/reference/api-surface.md",
  "docs/knowledge/architecture/nucleus-migration-map.md",
  "docs/knowledge/architecture/loophole-migration-map.md",
  "docs/knowledge/architecture/poodle-overlay-geometry-boundary.md",
  "docs/knowledge/architecture/settings-composition.md",
  "docs/knowledge/architecture/command-system-composition.md",
  "docs/knowledge/architecture/bridge-topology-composition.md",
  "docs/knowledge/architecture/history-composition.md",
  "docs/knowledge/architecture/secondary-consumer-migration-map.md",
  "docs/knowledge/architecture/package-topology.md",
  "docs/knowledge/contracts/012-distribution-and-compatibility.md",
];

test.each(publicationDocs)(
  "%s distinguishes current availability from private proofs",
  async (path) => {
    const prose = (await readRepoText(path)).replaceAll(/\s+/g, " ");
    expect(prose).not.toMatch(
      /\b(?:nothing is on npm|no npm\/crates\.io install|do not claim npm|registry today|private coordinated `0\.1\.0`|typescript packages remain private|publication (?:remains|is|are) deferred|publication itself is Card|registry-normalized [^.]* remains a release-lane gate)\b/i,
    );
    expect(prose).toMatch(/\]\((?:[^)\s]+\/)?(?:getting-started|release)\.md(?:#[^)]*)?\)/);
  },
);

test("vision uses the framework positioning and the single layout chain", async () => {
  const vision = (await readRepoText("docs/knowledge/vision.md")).replaceAll(/\s+/g, " ");
  expect(vision).toMatch(/Rust desktop application framework[^\]]*Tauri and GPUI/);
  expect(vision).toContain("(contracts/contract-index.md#positioning)");
  expect(vision).toContain("(contracts/002-composable-workspace-hosting.md#hosting-composition)");
  expect(vision).not.toMatch(
    /\bwindow\s*(?:->|→)\s*region\b|no-Surface composition|Shared Tauri Systems/i,
  );
});

test("the capability map includes the newer contracts and both window hosts", async () => {
  const suite = await readRepoText("docs/knowledge/architecture/shared-desktop-system-suite.md");
  for (const id of [
    "018",
    "019",
    "020",
    "021",
    "022",
    "023",
    "024",
    "025",
    "026",
    "027",
    "028",
    "029",
  ]) {
    expect(suite).toContain(`[${id}](../contracts/${id}-`);
  }
  expect(suite).toMatch(/window[^\n]*Tauri and GPUI/i);
  expect(suite).toContain("(../contracts/002-composable-workspace-hosting.md#hosting-composition)");
  expect(suite).not.toMatch(/optional layout-container host/i);
});

test("every registered contract is assigned a tier", async () => {
  const index = await readRepoText("docs/knowledge/contracts/contract-index.md");
  const tiers = index.split("## Contract Tiers")[1]?.split("## Contract Register")[0] ?? "";
  const registered = [...index.matchAll(/^\| \[(\d{3}) /gm)].map((match) => match[1]);
  const assigned = new Set(tiers.match(/\b\d{3}\b/g));
  expect(registered.length).toBeGreaterThan(0);
  expect(registered.filter((id) => !assigned.has(id ?? ""))).toEqual([]);
});

test("answered questions point to the owning record", async () => {
  const questions = await readRepoText("docs/knowledge/questions.md");
  const answers = questions
    .split(/^## /m)
    .flatMap((section) => section.split(/^Status: answered[^\n]*\n/m).slice(1));
  expect(answers.length).toBeGreaterThan(0);
  for (const answer of answers) {
    expect(answer.trim()).toMatch(/^Answer: (?:\[[^\]]+\]\([^)]+\)|Nucleus lane `[^\n`]+`)\.$/);
  }
});

test("held entries describe the implemented supervision and missing shutdown thread", async () => {
  const held = await readRepoText("docs/reference/held-surface.md");
  const bridge = held.split("\n").find((line) => line.startsWith("| bridge contract machinery"));
  const shutdown = held
    .split("\n")
    .find((line) => line.startsWith("| `AgentControlShutdownError::AlreadyShutdown`"));
  expect(bridge).toMatch(/Rust supervisor.*TypeScript supervision client/);
  expect(bridge).not.toMatch(/\bstub\b/i);
  expect(shutdown).toMatch(/no thread to join/);
  expect(shutdown).not.toMatch(/breaks? the public enum/i);
});

test("orientation points to the consumer roster and the canonical Rust routing table", async () => {
  const [readme, agents, scripts] = await Promise.all([
    readRepoText("README.md"),
    readRepoText("AGENTS.md"),
    readRepoText("scripts/README.md"),
  ]);
  for (const text of [readme, agents]) {
    expect(text).toContain("(docs/knowledge/architecture/repo-authority-map.md)");
    expect(text).not.toMatch(/\bfour consumers\b|\bfive sibling apps\b/i);
  }
  expect(agents).toContain("(scripts/README.md#rust-routing)");
  expect(agents).not.toContain("### Cargo entry points");
  expect(scripts).toContain("### Cargo entry points");
  expect(agents.split("<!-- oxide:begin -->")[0]).toMatch(
    /Oxide guidance below applies to direct Rust commands/,
  );
});

test("the knowledge base keeps task chronology and release history with their records", async () => {
  const knowledge = "docs/knowledge";
  const files = await readdir(new URL(`../${knowledge}/`, import.meta.url), { recursive: true });
  for (const path of files.filter((file) => file.endsWith(".md"))) {
    const content = await readRepoText(`${knowledge}/${path}`);
    if (/\bCards?\s+\d{2,}\b/i.test(content)) {
      throw new Error(`${knowledge}/${path} names a delivery card instead of its durable fact`);
    }
  }
  const release = await readRepoText(`${knowledge}/contracts/release.md`);
  expect(release).not.toMatch(/^## \d+\.\d+\.\d+ release/m);
  expect(release).toContain("(../../../CHANGELOG.md)");
});
