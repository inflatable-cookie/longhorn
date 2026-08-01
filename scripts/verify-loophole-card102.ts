import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

type RepositoryName = "loophole" | "poodle";

type FreezeFixture = {
  schema: string;
  outcome: string;
  donors: Record<
    RepositoryName,
    { commit: string; expected_clean: boolean; app_id?: string }
  >;
  source_checks: Array<{
    repository: RepositoryName;
    path: string;
    contains: string[];
    excludes: string[];
  }>;
  baseline_health: { outcome: string; blocker: string };
  storage: {
    canonical_app_id: string;
    stable_storage_name: string;
    profile_match: string;
    target_decision: string;
  };
  hierarchy: {
    canonical: string[];
    regional_surface_regions: string[];
    surface_habitats: string[];
    longhorn_fixture: string;
    longhorn_fixture_mapping: string;
  };
  history: {
    mutation_variants: number;
    mode: string;
    default_limit: number;
    automatic_group_window_ms: number;
    renderer_recent_entries: number;
  };
  admission: {
    donor_migration_writes: boolean;
    operator_gate: string;
  };
};

const longhornRoot = resolve(import.meta.dir, "..");
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/loophole-card102/behavior-freeze-v1.json",
    ),
    "utf8",
  ),
) as FreezeFixture;
const repositories: Record<RepositoryName, string> = {
  loophole: resolve(
    process.env.LOOPHOLE_REPO ?? resolve(longhornRoot, "../loophole"),
  ),
  poodle: resolve(
    process.env.POODLE_REPO ?? resolve(longhornRoot, "../poodle"),
  ),
};

assertEqual(
  fixture.schema,
  "longhorn.loophole-migration-freeze.v1",
  "fixture schema",
);
assertEqual(fixture.outcome, "pass_with_admission_gates", "fixture outcome");

for (const [name, donor] of Object.entries(fixture.donors) as Array<
  [RepositoryName, FreezeFixture["donors"][RepositoryName]]
>) {
  git(repositories[name], ["cat-file", "-e", `${donor.commit}^{commit}`]);
  assertEqual(
    git(repositories[name], ["status", "--porcelain"]) === "",
    donor.expected_clean,
    `${name} worktree state`,
  );
}

for (const check of fixture.source_checks) {
  const source = gitRead(
    repositories[check.repository],
    fixture.donors[check.repository].commit,
    check.path,
  );
  for (const token of check.contains) {
    assert(
      source.includes(token),
      `${check.repository}/${check.path} lost ${JSON.stringify(token)}`,
    );
  }
  for (const token of check.excludes) {
    assert(
      !source.includes(token),
      `${check.repository}/${check.path} gained ${JSON.stringify(token)}`,
    );
  }
}

verifyTauriIdentity();
verifyHierarchy();
verifyHistoryVocabulary();
verifyLonghornShape();

console.log(
  JSON.stringify(
    {
      schema: "longhorn.loophole-migration-freeze-verification.v1",
      outcome: fixture.outcome,
      donorCommits: {
        loophole: fixture.donors.loophole.commit,
        poodle: fixture.donors.poodle.commit,
      },
      sourceChecks: fixture.source_checks.length,
      hierarchy: fixture.hierarchy.canonical,
      regions: fixture.hierarchy.regional_surface_regions,
      storage: {
        canonicalAppId: fixture.storage.canonical_app_id,
        stableStorageName: fixture.storage.stable_storage_name,
        profileMatch: fixture.storage.profile_match,
        decision: fixture.storage.target_decision,
      },
      history: {
        mode: fixture.history.mode,
        mutationVariants: fixture.history.mutation_variants,
      },
      baselineHealth: fixture.baseline_health,
      donorMigrationWrites: fixture.admission.donor_migration_writes,
      operatorGate: fixture.admission.operator_gate,
    },
    null,
    2,
  ),
);

function verifyTauriIdentity(): void {
  const tauri = JSON.parse(
    gitRead(
      repositories.loophole,
      fixture.donors.loophole.commit,
      "aura/src-tauri/tauri.conf.json",
    ),
  ) as {
    identifier: string;
    app: {
      windows: Array<{ label: string; visible: boolean }>;
      security: { capabilities: Array<{ windows: string[] }> };
    };
  };
  assertEqual(
    tauri.identifier,
    fixture.storage.canonical_app_id,
    "Aura app id",
  );
  assertEqual(tauri.app.windows[0]?.label, "main", "boot window label");
  assertEqual(tauri.app.windows[0]?.visible, false, "hidden boot window");
  const trustedWindows = tauri.app.security.capabilities.flatMap(
    (capability) => capability.windows,
  );
  assert(trustedWindows.includes("main"), "main capability is missing");
  assert(
    trustedWindows.includes("workspace-*"),
    "workspace capability is missing",
  );
}

function verifyHierarchy(): void {
  assertEqual(
    JSON.stringify(fixture.hierarchy.canonical),
    JSON.stringify(["display", "window", "surface", "region", "panel"]),
    "canonical hierarchy",
  );
  assertEqual(
    fixture.hierarchy.regional_surface_regions.length,
    8,
    "regional Surface count",
  );
  assertEqual(
    JSON.stringify(fixture.hierarchy.surface_habitats),
    JSON.stringify(["regional", "focused-panel"]),
    "Surface habitats",
  );
}

function verifyHistoryVocabulary(): void {
  const source = gitRead(
    repositories.loophole,
    fixture.donors.loophole.commit,
    "pulse/crates/pulse-history/src/lib.rs",
  );
  const enumStart = source.indexOf("pub enum PulseHistoryMutation {");
  const enumEnd = source.indexOf("\n}\n\nimpl PulseHistoryMutation", enumStart);
  assert(
    enumStart >= 0 && enumEnd > enumStart,
    "history mutation enum not found",
  );
  const variants = source
    .slice(enumStart, enumEnd)
    .match(/^    [A-Z][A-Za-z0-9_]+\s*\{/gm)?.length;
  assertEqual(
    variants,
    fixture.history.mutation_variants,
    "history variant count",
  );
  assertEqual(fixture.history.default_limit, 100, "history default limit");
  assertEqual(fixture.history.automatic_group_window_ms, 750, "group window");
  assertEqual(
    fixture.history.renderer_recent_entries,
    8,
    "renderer history page",
  );
}

function verifyLonghornShape(): void {
  const source = readFileSync(
    resolve(longhornRoot, fixture.hierarchy.longhorn_fixture),
    "utf8",
  );
  const shape = JSON.parse(source) as {
    host_binding: { kind: string };
    definitions: { schema: { regions: unknown[] } };
  };
  assertEqual(shape.host_binding.kind, "surface", "Longhorn host binding");
  assertEqual(
    shape.definitions.schema.regions.length,
    8,
    "Longhorn fixture regions",
  );
  assertEqual(
    fixture.hierarchy.longhorn_fixture_mapping,
    "compatible-surface-shape-not-literal-current-donor",
    "Longhorn fixture mapping",
  );
}

function git(root: string, args: string[]): string {
  return execFileSync("git", ["-C", root, ...args], {
    encoding: "utf8",
  }).trim();
}

function gitRead(root: string, commit: string, path: string): string {
  return execFileSync("git", ["-C", root, "show", `${commit}:${path}`], {
    encoding: "utf8",
  });
}

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
  if (actual !== expected) {
    throw new Error(
      `${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}
