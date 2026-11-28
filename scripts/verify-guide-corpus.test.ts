import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
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
  const proofManifest = requireRecord(Bun.TOML.parse(manifestText), "proof manifest");
  const dependencies = requireRecord(proofManifest.dependencies, "proof dependencies");
  expect(dependencies).toHaveProperty("longhorn-update");
  expect(dependencies).not.toHaveProperty("longhorn-tauri-update");
  expect(guideText).toContain("(../../crates/longhorn-tauri-update/src/lib.rs)");
});
