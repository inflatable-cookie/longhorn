import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

type RepositoryName =
  | "soundcheck"
  | "soundcheck_library"
  | "signal"
  | "split-shell"
  | "jetstream"
  | "poodle";

type ConsumerName = "soundcheck" | "split-shell" | "jetstream";

type ConsumerFixture = {
  app_id: string;
  storage_profile: string | null;
  stable_storage_name: string | null;
  selected_typescript: string[];
  selected_rust: string[];
  forbidden_systems: string[];
};

type FreezeFixture = {
  schema: string;
  outcome: string;
  sources: {
    longhorn_compilation_commit: string;
    soundcheck_commit: string;
    soundcheck_library_commit: string;
    signal_commit: string;
    split-shell_commit: string;
    jetstream_commit: string;
    poodle_commit: string;
    poodle_prior_artifact_set: string;
  };
  branches: Record<RepositoryName, string>;
  split-shell_unrelated_dirty_paths: string[];
  consumers: Record<ConsumerName, ConsumerFixture>;
  authority: Record<string, string[]>;
  rollback: {
    retained_sources_required: boolean;
    exact_receipts_required: boolean;
    previous_locks_required: boolean;
    dual_write_authority: boolean;
    silent_fallback: boolean;
    automatic_source_cleanup: boolean;
  };
  publication: {
    package_managers: boolean;
    tags: boolean;
    hosted_releases: boolean;
  };
};

type SourceCheck = {
  repository: RepositoryName;
  path: string;
  contains: string[];
};

const longhornRoot = resolve(import.meta.dir, "..");
const fixture = JSON.parse(
  readFileSync(
    resolve(
      longhornRoot,
      "fixtures/migration/secondary-consumer-card113/behavior-authority-freeze-v1.json",
    ),
    "utf8",
  ),
) as FreezeFixture;

const repositories: Record<RepositoryName, string> = {
  soundcheck: resolve(
    process.env.SOUNDCHECK_REPO ?? resolve(longhornRoot, "../soundcheck"),
  ),
  soundcheck_library: resolve(
    process.env.SOUNDCHECK_LIBRARY_REPO ??
      resolve(longhornRoot, "../soundcheck-library"),
  ),
  signal: resolve(
    process.env.SIGNAL_REPO ?? resolve(longhornRoot, "../signal"),
  ),
  split-shell: resolve(
    process.env.SPLIT_SHELL_REPO ??
      resolve(longhornRoot, "../<private-consumer>"),
  ),
  jetstream: resolve(
    process.env.JETSTREAM_REPO ?? resolve(longhornRoot, "../jetstream"),
  ),
  poodle: resolve(
    process.env.POODLE_REPO ?? resolve(longhornRoot, "../poodle"),
  ),
};

const commits: Record<RepositoryName, string> = {
  soundcheck: fixture.sources.soundcheck_commit,
  soundcheck_library: fixture.sources.soundcheck_library_commit,
  signal: fixture.sources.signal_commit,
  split-shell: fixture.sources.split-shell_commit,
  jetstream: fixture.sources.jetstream_commit,
  poodle: fixture.sources.poodle_commit,
};

assertEqual(
  fixture.schema,
  "longhorn.secondary-consumer-behavior-authority-freeze.v1",
  "fixture schema",
);
assertEqual(fixture.outcome, "pass", "fixture outcome");
verifyRecoveryAndPublicationPolicy();
verifyRepositoryReceipts();
verifyPriorPoodleReceipt();
verifyApplicationIdentityAndLanes();
const sourceChecks = verifySourceBackedBehavior();
const selectedPaths = verifyLonghornPackageSelection();

console.log(
  JSON.stringify(
    {
      schema:
        "longhorn.secondary-consumer-behavior-authority-freeze-verification.v1",
      outcome: fixture.outcome,
      receipts: {
        longhornCompilation: fixture.sources.longhorn_compilation_commit,
        soundcheck: commits.soundcheck,
        soundcheckLibrary: commits.soundcheck_library,
        signal: commits.signal,
        split-shell: commits.split-shell,
        jetstream: commits.jetstream,
        poodle: commits.poodle,
        poodlePriorArtifactSet: fixture.sources.poodle_prior_artifact_set,
      },
      sourceChecks,
      selectedPackages: Object.fromEntries(
        Object.entries(fixture.consumers).map(([name, consumer]) => [
          name,
          {
            typescript: consumer.selected_typescript.length,
            rust: consumer.selected_rust.length,
            forbiddenOverlap: 0,
          },
        ]),
      ),
      selectedLonghornPaths: selectedPaths.length,
      retainedAuthorities: Object.fromEntries(
        Object.entries(fixture.authority).map(([name, entries]) => [
          name,
          entries.length,
        ]),
      ),
      overlap: {
        split-shellUnrelatedDirtyPaths: fixture.split-shell_unrelated_dirty_paths.length,
        migrationOwnedDirtyPaths: 0,
      },
      rollback: fixture.rollback,
      publication: fixture.publication,
    },
    null,
    2,
  ),
);

function verifyRecoveryAndPublicationPolicy(): void {
  assertEqual(
    fixture.rollback.retained_sources_required,
    true,
    "retained source requirement",
  );
  assertEqual(
    fixture.rollback.exact_receipts_required,
    true,
    "exact receipt requirement",
  );
  assertEqual(
    fixture.rollback.previous_locks_required,
    true,
    "previous lock requirement",
  );
  assertEqual(
    fixture.rollback.dual_write_authority,
    false,
    "dual-write authority",
  );
  assertEqual(fixture.rollback.silent_fallback, false, "silent fallback");
  assertEqual(
    fixture.rollback.automatic_source_cleanup,
    false,
    "automatic source cleanup",
  );
  assertEqual(
    fixture.publication.package_managers,
    false,
    "package-manager publication",
  );
  assertEqual(fixture.publication.tags, false, "tag publication");
  assertEqual(
    fixture.publication.hosted_releases,
    false,
    "hosted release publication",
  );
}

function verifyRepositoryReceipts(): void {
  git(longhornRoot, [
    "cat-file",
    "-e",
    `${fixture.sources.longhorn_compilation_commit}^{commit}`,
  ]);

  for (const name of Object.keys(repositories) as RepositoryName[]) {
    const root = repositories[name];
    assert(existsSync(root), `${name} repository is missing at ${root}`);
    assertEqual(
      git(root, ["rev-parse", "HEAD"]),
      commits[name],
      `${name} HEAD`,
    );
    assertEqual(
      git(root, ["branch", "--show-current"]),
      fixture.branches[name],
      `${name} branch`,
    );
    git(root, ["cat-file", "-e", `${commits[name]}^{commit}`]);
  }

  for (const name of [
    "soundcheck",
    "soundcheck_library",
    "signal",
    "jetstream",
    "poodle",
  ] as RepositoryName[]) {
    assertEqual(
      git(repositories[name], ["status", "--porcelain=v1"]),
      "",
      `${name} worktree`,
    );
  }

  const split-shellDirtyPaths = porcelainPaths(
    gitRaw(repositories.split-shell, ["status", "--porcelain=v1", "-z"]),
  );
  const expected = [...fixture.split-shell_unrelated_dirty_paths].sort();
  assertEqual(
    JSON.stringify(split-shellDirtyPaths),
    JSON.stringify(expected),
    "Split-shell unrelated dirty paths",
  );
  assert(
    split-shellDirtyPaths.every(
      (path) => path === "CHANGELOG.md" || path.startsWith("docs/"),
    ),
    "Split-shell dirty work overlaps runtime or package metadata",
  );
}

function verifyPriorPoodleReceipt(): void {
  const prior = JSON.parse(
    readFileSync(
      resolve(
        longhornRoot,
        "fixtures/migration/loophole-card112/conformance-closeout-v1.json",
      ),
      "utf8",
    ),
  ) as { sources: { poodle_commit: string; poodle_artifact_set: string } };
  assertEqual(
    prior.sources.poodle_commit,
    fixture.sources.poodle_commit,
    "prior Poodle commit",
  );
  assertEqual(
    prior.sources.poodle_artifact_set,
    fixture.sources.poodle_prior_artifact_set,
    "prior Poodle artifact set",
  );
}

function verifyApplicationIdentityAndLanes(): void {
  const configs: Array<{
    consumer: ConsumerName;
    repository: RepositoryName;
    path: string;
  }> = [
    {
      consumer: "soundcheck",
      repository: "soundcheck",
      path: "src-tauri/tauri.conf.json",
    },
    {
      consumer: "split-shell",
      repository: "split-shell",
      path: "src-tauri/tauri.conf.json",
    },
    {
      consumer: "jetstream",
      repository: "jetstream",
      path: "crates/jetstream-editor-tauri/tauri.conf.json",
    },
  ];
  for (const config of configs) {
    const tauri = JSON.parse(gitRead(config.repository, config.path)) as {
      identifier: string;
    };
    assertEqual(
      tauri.identifier,
      fixture.consumers[config.consumer].app_id,
      `${config.consumer} app id`,
    );
  }

  const lanes: Array<{
    repository: RepositoryName;
    token: string;
  }> = [
    { repository: "soundcheck", token: "g04-minimal-product-rebuild" },
    { repository: "split-shell", token: "g01.010" },
    { repository: "jetstream", token: "g05.008" },
  ];
  for (const lane of lanes) {
    assert(
      gitRead(lane.repository, "docs/README.md").includes(lane.token),
      `${lane.repository} active lane lost ${lane.token}`,
    );
  }

  assertEqual(
    fixture.consumers.soundcheck.storage_profile,
    "shared-product-root-v1",
    "Soundcheck storage profile",
  );
  assertEqual(
    fixture.consumers.soundcheck.stable_storage_name,
    "Soundcheck",
    "Soundcheck stable storage name",
  );
  assertEqual(
    fixture.consumers.split-shell.storage_profile,
    "native-platform-v1",
    "Split-shell storage profile",
  );
  assertEqual(
    fixture.consumers.split-shell.stable_storage_name,
    null,
    "Split-shell stable storage name",
  );
  assertEqual(
    fixture.consumers.jetstream.storage_profile,
    null,
    "Jetstream storage profile",
  );
}

function verifySourceBackedBehavior(): number {
  const checks: SourceCheck[] = [
    {
      repository: "soundcheck",
      path: "src-tauri/src/app_settings.rs",
      contains: [
        "WINDOW_SAVE_DEBOUNCE",
        "settings_path_from_database",
        "restore_main_window",
        "handle_main_window_event",
      ],
    },
    {
      repository: "soundcheck",
      path: "src/components/SettingsDialog.svelte",
      contains: [
        "Agent review",
        "Custom tags",
        "Backups",
        "Composer",
        "Keepsake",
      ],
    },
    {
      repository: "soundcheck",
      path: "src-tauri/src/library_commands.rs",
      contains: [
        "start_plugin_scan",
        "plugin_scan_status",
        "cancel_plugin_scan",
      ],
    },
    {
      repository: "soundcheck",
      path: "src-tauri/src/plugin_inspection_process.rs",
      contains: [
        "HELPER_FLAG",
        "current_exe",
        "parent_keepalive",
        "STARTUP_CANCELLATIONS",
      ],
    },
    {
      repository: "soundcheck",
      path: "src-tauri/src/plugin_inspection.rs",
      contains: [
        "WindowBuilder::new",
        "exit_inspection_helper_immediately",
        "processor.open",
      ],
    },
    {
      repository: "soundcheck",
      path: "package.json",
      contains: [
        '"@tauri-apps/api": "^2.11.1"',
        '"svelte": "^5.56.8"',
        '"typescript": "^6.0.3"',
      ],
    },
    {
      repository: "soundcheck",
      path: "src-tauri/Cargo.toml",
      contains: ['tauri = { version = "2.11.5"'],
    },
    {
      repository: "soundcheck_library",
      path: "crates/soundcheck-library-sqlite/src/lib.rs",
      contains: [
        "default_soundcheck_library_root_dir",
        '.join("Soundcheck")',
        "list_automatic_backups",
        "restore_automatic_backup",
        "validate_backup_database",
      ],
    },
    {
      repository: "soundcheck_library",
      path: "crates/soundcheck-library-jobs/src/lib.rs",
      contains: [
        "PluginScanJobSnapshot",
        "start_with_operation",
        "pub fn status",
        "pub fn cancel",
      ],
    },
    {
      repository: "split-shell",
      path: "src-tauri/src/workspace.rs",
      contains: [
        "PREFERENCES_FILE",
        "app_config_dir",
        "fs::write",
        "invalid_selection_does_not_replace_the_remembered_workspace",
        "presentation_save_is_bounded_and_preserves_workspace",
      ],
    },
    {
      repository: "split-shell",
      path: "src/App.svelte",
      contains: ["SplitView", "saveWorkspacePresentation"],
    },
    {
      repository: "split-shell",
      path: "tsconfig.json",
      contains: ["@inflatable-cookie/poodle-headless", "@inflatable-cookie/poodle-svelte/*"],
    },
    {
      repository: "split-shell",
      path: "vite.config.ts",
      contains: ["@inflatable-cookie/poodle-headless", "@inflatable-cookie/poodle-svelte"],
    },
    {
      repository: "split-shell",
      path: "src-tauri/src/lib.rs",
      contains: [
        "mod repository",
        "mod search",
        "mod editorial",
        "mod git",
        "read_content",
        "load_editorial_dashboard",
        "sync_git_workspace",
      ],
    },
    {
      repository: "split-shell",
      path: "package.json",
      contains: [
        '"@tauri-apps/api": "^2.11.1"',
        '"svelte": "^5.56.8"',
        '"typescript": "^6.0.3"',
      ],
    },
    {
      repository: "split-shell",
      path: "src-tauri/Cargo.toml",
      contains: ['tauri = { version = "2.11.5"'],
    },
    {
      repository: "jetstream",
      path: "editor-ui/src/bridge.ts",
      contains: [
        "jetstream:state",
        "page:ready",
        "reportViewport",
        "forwardViewportInput",
      ],
    },
    {
      repository: "jetstream",
      path: "editor-ui/src/shortcuts.ts",
      contains: ["commandFor", "installShortcuts", "send(command)"],
    },
    {
      repository: "jetstream",
      path: "crates/jetstream-editor-tauri/src/publish.rs",
      contains: ["StatePublisher", "resync"],
    },
    {
      repository: "jetstream",
      path: "crates/jetstream-editor-tauri/src/surface.rs",
      contains: [
        "addSubview_positioned_relativeTo",
        "GpuContext::embedded",
        "std::mem::forget(view)",
      ],
    },
    {
      repository: "jetstream",
      path: "crates/jetstream-editor-tauri/src/lib.rs",
      contains: ["surface::attach", "WindowEvent::Destroyed", "render_loop"],
    },
    {
      repository: "jetstream",
      path: "editor-ui/package.json",
      contains: ['"@tauri-apps/api": "^2.11.1"', '"svelte": "^5.38.6"'],
    },
    {
      repository: "jetstream",
      path: "crates/jetstream-editor-tauri/Cargo.toml",
      contains: ['tauri = { version = "2"', "macos-private-api"],
    },
  ];

  for (const check of checks) {
    const source = gitRead(check.repository, check.path);
    for (const token of check.contains) {
      assert(
        source.includes(token),
        `${check.repository}/${check.path} lost ${JSON.stringify(token)}`,
      );
    }
  }
  return checks.length;
}

function verifyLonghornPackageSelection(): string[] {
  const selectedPaths = new Set<string>();
  for (const [consumerName, consumer] of Object.entries(
    fixture.consumers,
  ) as Array<[ConsumerName, ConsumerFixture]>) {
    for (const packageName of consumer.selected_typescript) {
      const directory = packageName.replace("@inflatable-cookie/longhorn-", "");
      const path = `packages/${directory}`;
      const manifest = JSON.parse(
        readFileSync(resolve(longhornRoot, path, "package.json"), "utf8"),
      ) as { name: string };
      assertEqual(manifest.name, packageName, `${consumerName} ${packageName}`);
      selectedPaths.add(path);
    }
    for (const crateName of consumer.selected_rust) {
      const path = `crates/${crateName}`;
      const manifest = readFileSync(
        resolve(longhornRoot, path, "Cargo.toml"),
        "utf8",
      );
      assert(
        manifest.includes(`name = \"${crateName}\"`),
        `${consumerName} selected crate ${crateName} is missing`,
      );
      selectedPaths.add(path);
    }
    for (const forbidden of consumer.forbidden_systems) {
      const collisions = [
        ...consumer.selected_typescript,
        ...consumer.selected_rust,
      ].filter((name) => packageMatchesSystem(name, forbidden));
      assertEqual(
        collisions.length,
        0,
        `${consumerName} forbidden ${forbidden} package overlap`,
      );
    }
  }

  const paths = [...selectedPaths].sort();
  const changedSinceCompilation = git(longhornRoot, [
    "diff",
    "--name-only",
    `${fixture.sources.longhorn_compilation_commit}..HEAD`,
    "--",
    ...paths,
  ]);
  assertEqual(
    changedSinceCompilation,
    "",
    "selected Longhorn packages changed after compilation",
  );
  const dirtySelectedPaths = git(longhornRoot, [
    "status",
    "--porcelain=v1",
    "--",
    ...paths,
  ]);
  assertEqual(dirtySelectedPaths, "", "selected Longhorn package worktree");
  return paths;
}

function packageMatchesSystem(name: string, system: string): boolean {
  const normalized = name
    .replace("@inflatable-cookie/longhorn-", "")
    .replace("longhorn-", "")
    .replace(/^tauri-/, "");
  const aliases: Record<string, string[]> = {
    commands: ["command", "commands"],
    surfaces: ["surfaces", "surface-windowing", "surface-transfer"],
    "backing-surface": ["native-content-backing-surface"],
    "isolated-window": ["native-content-isolated-window"],
    "child-view": ["native-content-child-view"],
  };
  return (aliases[system] ?? [system]).some(
    (marker) => normalized === marker || normalized.endsWith(`-${marker}`),
  );
}

function porcelainPaths(source: string): string[] {
  if (source === "") return [];
  return source
    .split("\0")
    .filter((line) => line.length > 0)
    .map((line) => line.slice(3))
    .map((path) => (path.includes(" -> ") ? path.split(" -> ")[1]! : path))
    .sort();
}

function git(root: string, args: string[]): string {
  return execFileSync("git", ["-C", root, ...args], {
    encoding: "utf8",
  }).trim();
}

function gitRaw(root: string, args: string[]): string {
  return execFileSync("git", ["-C", root, ...args], {
    encoding: "utf8",
  });
}

function gitRead(repository: RepositoryName, path: string): string {
  return execFileSync(
    "git",
    ["-C", repositories[repository], "show", `${commits[repository]}:${path}`],
    { encoding: "utf8" },
  );
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
