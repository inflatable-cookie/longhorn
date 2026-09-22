# Changelog

All notable Longhorn changes are recorded here. A release tags the repository
and publishes the three TypeScript packages to npm; consumers take the Rust
crates by git tag and the packages by version.

## [Unreleased]

### Changed
- **The agent-control server is a compile-time opt-in a packaged consumer may enable.**
  `longhorn-tauri-agent-control`'s `dev` feature is now `agent-control`. A
  build without it still contains none of the surface. `evaluate` sits behind
  a second feature, `agent-control-evaluate`, off by default and expected
  only in dev/test; a packaged `agent-control` build answers typed
  `Unsupported`. Loopback binding, the per-instance bearer token, and Origin
  validation are unchanged. Contract 022 amended 2026-09-22.
- **The update install is staged, and progress is observable while it runs.**
  `UpdateController::install` becomes `prepare` (fetch, verify, retain an
  identity-bound staged artifact), `apply` (replace the application with it),
  and `cancel` (discard it). `ReadyToInstall` is a retained state that survives
  a deferred "Later" install, and byte progress is published out of band on
  `UpdateProgressEvent` (`longhorn://update/progress`) rather than only when the
  call returns. Verification is unchanged: `apply` still accepts only a
  `VerifiedArtifact`, and a failed verification discards rather than stages.
  The Tauri install permission grants `longhorn_update_prepare` and
  `longhorn_update_apply`; `longhorn_update_cancel` sits with the other local
  mutations. Contract 018 amended 2026-09-22.

## [0.1.0] - 2026-09-16

### Added
- Strict-ready Northstar documentation spine.
- Initial five-application Tauri extraction audit.
- Composable Rust and TypeScript systems for storage, backup/recovery,
  display/window hosting, layout, optional Surfaces and transfer, settings,
  commands/keymaps/palette, bridge topology, linear history, operations,
  notifications, and native-content coordination.
- Svelte lifecycle and public-Poodle composition adapters.
- Minimal, Surface-free workspace, full-hosting, and optional-server greenfield
  examples.
- Checked adoption guides and generated API inventory.
- Generation-checked, policy-admitted retained child-view navigation with
  exact native receipts and packaged macOS evidence.
- Explicit present/absent grouped-adapter restore evidence with zero-payload
  deletion, restart-safe rollback-to-absence, and per-domain receipt evidence.
- Optional production fork-tree history layer (`longhorn-history-tree`,
  `longhorn-tauri-history-tree`, `@inflatable-cookie/longhorn-history-tree`) behind the linear
  adoption checkpoint.
- Process-wide best-effort diagnostics seam
  (`longhorn_core::install_best_effort_diagnostics`) observing tolerated
  event-emit, adapter-teardown, and journal-cleanup failures.

### Changed
- **Collapsed the eighteen TypeScript packages into three**, grouped by peer
  requirement rather than by domain: `@inflatable-cookie/longhorn` (no peers),
  `@inflatable-cookie/longhorn-poodle-svelte`, and
  `@inflatable-cookie/longhorn-tauri`. Every domain is now a subpath. 61 entry
  points became 62 and nothing that resolved before stopped resolving, but
  every import specifier changes: `@inflatable-cookie/longhorn-core` is
  `@inflatable-cookie/longhorn/core`, `longhorn-settings/poodle` is
  `longhorn-poodle-svelte/settings/poodle`, and `longhorn-bridge/tauri` is
  `longhorn-tauri/bridge`. Consumers migrate at their next uptake.
- Migrated Nucleus, Loophole, Soundcheck, Split-shell, and Jetstream onto selected
  shared systems while retaining product authority downstream.
- Standardized canonical-id storage defaults, stable storage-name overrides,
  profile transitions, backup, restore, and receipt-bound cleanup.
- The deterministic private `0.1.0` candidate at Card 127 bound 17 TypeScript
  packages and 36 Rust crates across five exact Poodle artifacts and seven consumer
  graphs. The tree now produces three TypeScript packages and 49 Rust crates:
  the TypeScript packages publish to npm, and the Rust crates set
  `publish = false` and are taken by git tag.

### Fixed
- Layout ratios validate on deserialization; sizing bounds above 100% are
  unrepresentable.
- Window lifecycle: event-thread flush deferral, shared cancelable timer
  wakes, coherent retag state migration, typed install-label validation, and
  closure of the recorded reveal/retained-normal/destroy races.
- Transfer: truthful `session_consumed` aborts, post-publication
  reconciliation evidence instead of asserts, snapshot/destroy client-slot
  race closure, and epoch-ordered client-changed events.
- Storage: all 22 config/settings/command Tauri commands run off the main
  thread; bare loads self-heal terminal restore journals.
