import { execFileSync, spawnSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const longhornRoot = resolve(import.meta.dir, "..");
const nucleusRoot = resolve(
  process.env.NUCLEUS_REPO ?? resolve(longhornRoot, "../nucleus"),
);
const poodleRoot = resolve(longhornRoot, "../poodle");
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/nucleus-card101/conformance-closeout-v1.json",
    ),
    "utf8",
  ),
) as Card101Fixture;

verifyFixture();
verifySourceReceipts();
const poodleEvidencePath = findPoodleEvidence();
const artifactProof = verifyArtifacts(poodleEvidencePath);
const priorTraces = verifyPriorTraces();
const inventory = verifyCurrentComposition();
verifyRestartMatrix();

console.log(
  JSON.stringify(
    {
      schema: fixture.schema,
      outcome: "pass",
      sources: fixture.sources,
      poodleEvidencePath,
      poodleArtifactSet: artifactProof.poodleArtifactSet,
      priorTraces,
      profiles: fixture.profiles,
      rollback: fixture.rollback,
      inventory,
      retainedPolicy: [
        "desktop_profile",
        "storage_migration",
        "window_host",
        "workspace_ui",
        "browser_panel",
      ],
      cleanup: fixture.cleanup,
      packageManagerPublication: false,
    },
    null,
    2,
  ),
);

function verifyFixture(): void {
  assertEqual(
    fixture.schema,
    "longhorn.nucleus-migration-conformance-closeout.v1",
    "fixture schema",
  );
  assertEqual(fixture.outcome, "pass", "fixture outcome");
  assertEqual(
    fixture.rollback.database_before_sha256,
    fixture.rollback.database_after_sha256,
    "rollback database digest",
  );
  assertEqual(
    fixture.rollback.current_evidence_sha256,
    fixture.rollback.previous_evidence_sha256,
    "rollback evidence digest",
  );
  assert(!fixture.rollback.dual_write, "rollback used dual-write");
  assert(!fixture.rollback.silent_fallback, "rollback used silent fallback");
  assert(fixture.rollback.temporary_worktree_removed, "rollback worktree retained");
  assert(fixture.rollback.temporary_profile_removed, "rollback profile retained");
  assert(!fixture.cleanup.automatic_deletion, "legacy cleanup was automatic");
  assert(
    fixture.cleanup.eligibility_requires_exact_import_receipt,
    "cleanup is not receipt-bound",
  );
  assert(
    !fixture.cleanup.operator_cleanup_authority_granted,
    "cleanup authority was fabricated",
  );
  assert(!fixture.composition.package_manager_publication, "package publication");
}

function verifySourceReceipts(): void {
  for (const [label, root, commit] of [
    ["Poodle", poodleRoot, fixture.sources.poodle_commit],
    ["Nucleus", nucleusRoot, fixture.sources.nucleus_commit],
  ] as const) {
    assertEqual(git(root, ["rev-parse", "HEAD"]), commit, `${label} HEAD`);
    git(root, ["cat-file", "-e", `${commit}^{commit}`]);
  }
  assertClean("Poodle", poodleRoot);
  assertClean("Nucleus", nucleusRoot);

  const selectedLonghorn = [
    "Cargo.toml",
    "Cargo.lock",
    "packages/core",
    "packages/config",
    "packages/layout",
    "packages/svelte",
    "packages/poodle",
    "packages/native-content",
    "packages/native-content-svelte",
    "packages/tauri",
    "crates/longhorn-core",
    "crates/longhorn-config",
    "crates/longhorn-tauri-config",
    "crates/longhorn-display",
    "crates/longhorn-windowing",
    "crates/longhorn-tauri-windowing",
    "crates/longhorn-windowing-config",
    "crates/longhorn-layout",
    "crates/longhorn-layout-config",
    "crates/longhorn-native-content",
    "crates/longhorn-tauri-native-content-child-view",
  ];
  git(longhornRoot, [
    "cat-file",
    "-e",
    `${fixture.sources.longhorn_selected_source_commit}^{commit}`,
  ]);
  const selectedDiff = spawnSync(
    "git",
    [
      "diff",
      "--quiet",
      fixture.sources.longhorn_selected_source_commit,
      "HEAD",
      "--",
      ...selectedLonghorn,
    ],
    { cwd: longhornRoot, stdio: "ignore" },
  );
  assert(
    selectedDiff.status === 0,
    "selected Longhorn sources differ from the admitted source commit",
  );
  const status = git(longhornRoot, [
    "status",
    "--porcelain",
    "--",
    ...selectedLonghorn,
  ]);
  assert(!status, `selected Longhorn sources are dirty:\n${status}`);
  git(nucleusRoot, [
    "merge-base",
    "--is-ancestor",
    fixture.sources.nucleus_previous_authority_commit,
    fixture.sources.nucleus_commit,
  ]);
}

function findPoodleEvidence(): string {
  const artifactsRoot = resolve(poodleRoot, ".artifacts");
  for (const entry of readdirSync(artifactsRoot, { withFileTypes: true })) {
    if (!entry.isDirectory() || !entry.name.startsWith("svelte-pack-install-")) {
      continue;
    }
    const path = resolve(artifactsRoot, entry.name, "evidence.json");
    try {
      const evidence = JSON.parse(readFileSync(path, "utf8"));
      if (evidence.artifactSetId === fixture.sources.poodle_artifact_set) {
        return path;
      }
    } catch {
      // Ignore incomplete historical proof directories.
    }
  }
  throw new Error(
    `Poodle artifact set ${fixture.sources.poodle_artifact_set} is missing; run effigy test:svelte-pack-install in Poodle`,
  );
}

function verifyArtifacts(poodleEvidencePath: string): ArtifactProof {
  const output = command(
    longhornRoot,
    ["bun", "scripts/verify-nucleus-private-artifacts.ts"],
    {
      POODLE_EVIDENCE_PATH: poodleEvidencePath,
      EXPECTED_POODLE_COMMIT: fixture.sources.poodle_commit,
      EXPECTED_POODLE_ARTIFACT_SET: fixture.sources.poodle_artifact_set,
      EXPECTED_NUCLEUS_COMMIT: fixture.sources.nucleus_commit,
    },
  );
  const proof = JSON.parse(output) as ArtifactProof;
  assertEqual(proof.outcome, "pass", "private artifact proof");
  assertEqual(
    proof.poodleArtifactSet,
    fixture.sources.poodle_artifact_set,
    "Poodle artifact set",
  );
  assert(!proof.audits.packageManagerPublication, "artifact proof published packages");
  assert(proof.audits.oneSvelteRuntime, "artifact proof has duplicate Svelte");
  assert(!proof.audits.surfacesInConsumerGraph, "artifact proof resolved Surfaces");
  return proof;
}

function verifyPriorTraces(): string[] {
  const cards = ["096", "097", "098", "099", "100"];
  for (const card of cards) {
    const proof = JSON.parse(
      command(longhornRoot, ["bun", `scripts/verify-nucleus-card${card}.ts`]),
    );
    assertEqual(proof.outcome, "pass", `Card ${card} trace`);
  }
  return cards;
}

function verifyCurrentComposition(): Record<string, unknown> {
  const commit = fixture.sources.nucleus_commit;
  const manifest = JSON.parse(
    nucleusRead(commit, "apps/desktop/package.json"),
  ) as PackageManifest;
  const requiredRendererPackages = [
    "@longhorn/core",
    "@longhorn/layout",
    "@longhorn/native-content",
    "@longhorn/native-content-svelte",
    "@longhorn/poodle",
    "@longhorn/svelte",
  ];
  for (const name of requiredRendererPackages) {
    assert(
      manifest.dependencies[name]?.startsWith("file:../../../longhorn/"),
      `${name} does not resolve from the admitted private source`,
    );
  }
  for (const forbidden of ["@longhorn/surfaces", "@longhorn/surface-transfer"]) {
    assert(!manifest.dependencies[forbidden], `manifest depends on ${forbidden}`);
  }

  const lock = nucleusRead(commit, "apps/desktop/bun.lock");
  for (const forbidden of ["@longhorn/surfaces", "@longhorn/surface-transfer"]) {
    const resolvedEntry = new RegExp(`^\\s{4}${JSON.stringify(forbidden)}\\s*:`, "m");
    assert(!resolvedEntry.test(lock), `renderer lock resolves ${forbidden}`);
  }

  const cargoTree = command(nucleusRoot, [
    "cargo",
    "tree",
    "-p",
    "nucleus-desktop",
    "--edges",
    "normal",
    "--prefix",
    "none",
  ]);
  for (const forbidden of [
    "longhorn-surfaces ",
    "longhorn-surface-windowing ",
    "longhorn-surface-transfer ",
  ]) {
    assert(!cargoTree.includes(forbidden), `Rust graph resolves ${forbidden.trim()}`);
  }

  const config = JSON.parse(
    nucleusRead(commit, "apps/desktop/src-tauri/tauri.conf.json"),
  );
  assertEqual(config.identifier, "com.inflatablecookie.nucleus", "Tauri identifier");
  const capability = config.app.security.capabilities[0];
  assertEqual(capability.webviews, ["main"], "controller webviews");
  assert(!("remote" in capability), "remote capability selector exists");
  assert(
    capability.permissions.every(
      (permission: string) => !permission.toLowerCase().includes("webview"),
    ),
    "raw Webview capability remains",
  );

  for (const token of [
    'from "@tauri-apps/api/webview"',
    "new Webview",
    "browser_panel_ensure",
    "browser_panel_set_bounds",
    "WindowGeometryWorker",
    "NUCLEUS_DESKTOP_DATA_ROOT",
    "load_workspace_ui_config",
    "save_workspace_ui_config",
  ]) {
    const matches = gitGrep(nucleusRoot, commit, token, [
      "apps/desktop/src",
      "apps/desktop/src-tauri/src",
    ]);
    const productionMatches = matches
      .split("\n")
      .filter(Boolean)
      .filter(
        (line) =>
          !line.includes("/tests/") &&
          !line.includes(".test.") &&
          !line.includes(".vitest."),
      );
    assert(
      productionMatches.length === 0,
      `superseded active mechanism remains: ${token}\n${productionMatches.join("\n")}`,
    );
  }
  assert(
    !gitPathExists(nucleusRoot, commit, "apps/desktop/src/lib/workspaceUi.ts"),
    "superseded renderer workspace store remains",
  );

  const legacySurfaceUses = gitGrep(nucleusRoot, commit, "LegacySurface", [
    "apps/desktop/src-tauri/src",
  ]);
  assert(legacySurfaceUses, "legacy Surface decoder classification is missing");
  assert(
    legacySurfaceUses
      .split("\n")
      .every((line) => line.includes("workspace_ui/legacy.rs")),
    `legacy Surface type escaped its decoder:\n${legacySurfaceUses}`,
  );
  const activeSurfaceIdUses = gitGrep(nucleusRoot, commit, "active_surface_id", [
    "apps/desktop/src-tauri/src",
  ]);
  assert(
    activeSurfaceIdUses.split("\n").every(
      (line) =>
        line.includes("workspace_ui/legacy.rs") || line.includes("tests/panel_guards.rs"),
    ),
    `legacy Surface state escaped decoder/guard scope:\n${activeSurfaceIdUses}`,
  );

  return {
    requiredRendererPackages,
    surfacePackages: false,
    surfaceTypes: false,
    surfaceState: false,
    surfaceCommands: false,
    remoteCapability: false,
    rawWebviewCapability: false,
    poodleOverlayGeometry: true,
    legacySurfaceDecoderOnly: true,
  };
}

function verifyRestartMatrix(): void {
  for (const name of Object.values(fixture.profiles)) {
    if (typeof name !== "string") continue;
    command(nucleusRoot, [
      "cargo",
      "test",
      "-p",
      "nucleus-desktop",
      name.split("::").at(-1)!,
      "--",
      "--nocapture",
    ]);
  }
}

function assertClean(label: string, root: string): void {
  const status = git(root, ["status", "--porcelain"]);
  assert(!status, `${label} worktree is dirty:\n${status}`);
}

function nucleusRead(commit: string, path: string): string {
  return git(nucleusRoot, ["show", `${commit}:${path}`]);
}

function gitPathExists(root: string, commit: string, path: string): boolean {
  return spawnSync("git", ["cat-file", "-e", `${commit}:${path}`], {
    cwd: root,
    stdio: "ignore",
  }).status === 0;
}

function gitGrep(
  root: string,
  commit: string,
  token: string,
  paths: string[],
): string {
  const result = spawnSync("git", ["grep", "-n", "-F", token, commit, "--", ...paths], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status === 1) return "";
  if (result.status !== 0) throw new Error(result.stderr || `git grep failed for ${token}`);
  return result.stdout.trim();
}

function git(root: string, arguments_: string[]): string {
  return execFileSync("git", arguments_, {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function command(
  root: string,
  arguments_: string[],
  environment: Record<string, string> = {},
): string {
  return execFileSync(arguments_[0], arguments_.slice(1), {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, ...environment },
    maxBuffer: 20 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
    );
  }
}

interface PackageManifest {
  readonly dependencies: Record<string, string>;
}

interface ArtifactProof {
  readonly outcome: string;
  readonly poodleArtifactSet: string;
  readonly audits: {
    readonly packageManagerPublication: boolean;
    readonly oneSvelteRuntime: boolean;
    readonly surfacesInConsumerGraph: boolean;
  };
}

interface Card101Fixture {
  readonly schema: string;
  readonly outcome: string;
  readonly sources: {
    readonly longhorn_selected_source_commit: string;
    readonly poodle_commit: string;
    readonly poodle_artifact_set: string;
    readonly nucleus_commit: string;
    readonly nucleus_previous_authority_commit: string;
  };
  readonly profiles: Record<string, string | boolean>;
  readonly rollback: Record<string, string | boolean> & {
    readonly database_before_sha256: string;
    readonly database_after_sha256: string;
    readonly current_evidence_sha256: string;
    readonly previous_evidence_sha256: string;
    readonly dual_write: boolean;
    readonly silent_fallback: boolean;
    readonly temporary_worktree_removed: boolean;
    readonly temporary_profile_removed: boolean;
  };
  readonly composition: Record<string, boolean | string[]> & {
    readonly package_manager_publication: boolean;
  };
  readonly cleanup: Record<string, boolean> & {
    readonly automatic_deletion: boolean;
    readonly eligibility_requires_exact_import_receipt: boolean;
    readonly operator_cleanup_authority_granted: boolean;
  };
}
