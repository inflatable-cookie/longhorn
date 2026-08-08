# Changelog

All notable Longhorn changes are recorded here. Releases are source-only
annotated Git tags from the canonical repository; consumers depend on them
with `git` + `tag` dependencies.

## [Unreleased]

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
  `longhorn-tauri-history-tree`, `@longhorn/history-tree`) behind the linear
  adoption checkpoint.
- Process-wide best-effort diagnostics seam
  (`longhorn_core::install_best_effort_diagnostics`) observing tolerated
  event-emit, adapter-teardown, and journal-cleanup failures.

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

### Changed

- Migrated Nucleus, Loophole, Soundcheck, Bovine, and Jetstream onto selected
  shared systems while retaining product authority downstream.
- Standardized canonical-id storage defaults, stable storage-name overrides,
  profile transitions, backup, restore, and receipt-bound cleanup.

- Card 127 produced the deterministic private `0.1.0` candidate binding 17 TypeScript
  packages, 36 Rust crates, five exact Poodle artifacts, and seven consumer
  graphs. Package-manager publication, registry ownership, and hosted
  releases remain deferred.
