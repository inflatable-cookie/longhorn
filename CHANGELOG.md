# Changelog

All notable Longhorn changes are recorded here. A release tags the repository
and publishes the three TypeScript packages to npm; consumers take the Rust
crates by git tag and the packages by version.

## [Unreleased]

## [0.6.0] - 2026-11-23

### Breaking
- **Tauri adapters: one authority trait per adapter (`longhorn-tauri-*`).**
  The mirroring `*Service` traits are removed: `BridgeCommandService`,
  `CommandHostService`, `ConfigOperationsCommandService`,
  `ForkHistoryHostService`, `HistoryHostService`, `LicenceHostService`,
  `NotificationHostService`, `OperationHostService`,
  `SettingsCommandService`, `UpdateHostService`, `TransferCommandService` and
  `SurfaceTransferCommandService`. Authorities are named `<Domain>HostAuthority`:
  `ConfigOperationsAuthority` becomes `ConfigOperationsHostAuthority` and
  `SettingsAuthority` becomes `SettingsHostAuthority`. `Tauri*State::new` takes
  the handler assembly directly. Migration for config: replace
  `ConfigOperationsCommandService` with `ConfigOperationsHostAuthority`
  (changing `&self` receivers to `&mut self`, or wrapping the state in
  `ConfigOperationsHandlerAssembly::new(authority)`), pass
  `Arc::new(ConfigOperationsHandlerAssembly::new(authority))` to
  `TauriConfigOperationsState::new`, and rename the `*Authority` references.
  Transfer: `PanelTransferAdapter` and `SurfaceTransferAdapter` no longer
  receive `&mut TransferCoordinator`. Create sessions in `start_*` through
  `TransferSessionRegistrar::create_session`, with
  `prepare_panel_session_source` or `prepare_surface_session_source`, and accept
  `TerminalTransferAttempt` / `TerminalTransferResolution` in `commit_*`, with
  `commit_resolved_panel_transfer` or `commit_resolved_surface_transfer`.
  `TransferHandlerError::Projection` is removed, and transfer commands return
  typed `TransferHostError` payloads instead of `String`.
  `BridgeHandlerAssembly::teardown` returns `Result<(), BridgeHostError>`
  instead of silently skipping on a poisoned mutex, so callers must handle the
  result. `TransferHandlerAssembly::project_selector` is now `pub(crate)` and
  has no public replacement.
- **Licence: product binding is required (`longhorn-licence`).** `verify`,
  `SignedFileSource::new` and `TokenRedemptionSource::new` take the expected
  product, and a mismatch returns `VerificationError::ProductMismatch`. The
  `asserted_remotely` alias is removed: use
  `VerifiedLicence::from_remote_assertion`. Contract 019 documents the binding.
- **Surface: one validator, one limits type, one config stack
  (`longhorn-surfaces`, `longhorn-surfaces-config`).**
  - `LayoutLimits` is replaced by `SurfaceLimits`, built with
    `SurfaceLimits::builder()`. `RegisteredLayoutDomain` and its sibling
    duplicates are removed; use `RegisteredSurfaceDomain`,
    `PersistedSurfaceDocument`, `SurfaceMigration`,
    `LegacySurfaceFormatMigration` and `SurfaceBackupPolicy`.
  - `validate_document`, `normalize_document`,
    `validate_normalized_document`, `validate_normalized_layout`,
    `resolve_surfaces`, `compose_surface_window_plan`,
    `SurfaceMutationEngine::new` and `RegisteredSurfaceDomain::new` bind
    `&LayoutDefinitionRegistry`.
  - `PersistedSurfaceDocument` carries `registryDigest`. Consumers with
    pre-unification digest-less Surface files, or layout files written under an
    earlier digest, bump their `DomainDescriptor` schema version, attach
    `LegacySurfaceFormatMigration::digestless` or
    `LegacySurfaceFormatMigration::with_expected_digest` (or a custom
    `SurfaceMigration`), and rewrite through
    `ConfigStore::rewrite_migrated_domain` after a pre-migration safety backup.
  - `debounced_layout_presentation` binds presentation debounce lanes to their
    `RegisteredSurfaceDomain`'s registry. It replaces
    `LayoutPresentationStrategy::new`.
  - `publish_surface_mutation` and `commit_surface_transfer` drop their
    per-call registry argument. `admit_panel_session` and
    `commit_panel_transfer` take `&RegisteredSurfaceDomain<M>`.
  - `PanelInstancePolicy::OnePerContainer` is renamed
    `PanelInstancePolicy::OnePerSurface` (`"one_per_surface"` on the wire)
    across Rust, the generated TypeScript bindings and
    `@inflatable-cookie/longhorn-poodle-svelte`.
  - Behaviour changes that callers see: `CloseSurface` removes the closed
    Surface's regions, sizing slots and placed panel instances in the same
    commit; `DuplicateSurface` preserves `SurfacePresentation`; the
    `MoveSurface` active-Surface fallback uses the primary-member index.
- **History, operation and notification kernels.**
  `ForkHistory::record_applied` derives encoded weight from `HistoryPolicy`:
  `ForkRecord` drops `encoded_weight`, and `ForkHistoryError::Policy` is new.
  `ForkLoadError` splits `PayloadMigration` from `Payload`.
  `NotificationLedger::dismiss` takes `NotificationDismiss`, and
  `NotificationPublishOutcome::AlreadyPublished` carries `notification_id` and
  `record: Option<NotificationRecord>`. `observe_terminal_operation` and
  `observe_terminal_receipt` accept `OperationTerminalEvidence`.
  `OperationCatalogue::request_cancellation` moves queued work to `Cancelling`
  for executor dispatch unless `QueuedCancellationConfirmation::ConfirmedUnstarted`
  is supplied.
- **Update native content (`longhorn-update`).** `PreparedTransfer::new`
  allocates a transfer identity and is no longer callable in a `const` context;
  call it at runtime. `StagedArtifact::new` takes the channel and the verified
  artifact, and derives its version from the artifact. Apply commands require
  the observed staged digest.
- **PTY `Failure` (`longhorn-pty`).** OS-backed `Failure` variants are now struct
  variants carrying `raw_os_error`. Match the struct form, or read the code
  through `Failure::raw_os_error()`.
- **Core bridge command bindings.** Plain host services are no longer `Copy`,
  `Clone` or comparable; each issues a unique request ID. Share a service by
  reference or behind an `Arc`. Command revision and registry generation
  advances, and bridge stream sequence advances, return a typed overflow error
  that callers must handle. Shared core revisions and counters reject values
  outside JavaScript's safe-integer range.
- **Transfer and surface transfer (`longhorn-transfer`,
  `longhorn-surface-transfer`).** `TransferClientSnapshot` removes the always-`None`
  `current_lease_generation` field; use `TransferCoordinator::current_lease_generation`.
  `DropZone` and `ClientDropZone` derive `accepted_capability()` from
  `target.capability()` and no longer store a duplicate field. `TransferAbort`
  gains `layout_code` and getters, including `retryable()`, on both
  `TransferAbort` and `SurfaceTransferAbort`. Code that builds either struct
  literally must supply the new field.
- **Git staging (`longhorn-git-workspace`).** `remove_owned_staging` is removed,
  and `create_owned_staging` returns an `OwnedStaging` handle instead of a
  `PathBuf`. Hold the handle and pass `staging.path()` where the staging path
  was passed, then call `staging.remove()` where `remove_owned_staging(&staging)`
  was called. `OwnedStaging::remove` returns the operating system's error
  instead of dropping it. Contract 029 matches.
- **Git credentials (`longhorn-git-workspace`).** `credential_bridge_configured`
  is removed; use `credential.is_some()`. Apps that apply `env_vars()` to their
  own Git commands must clear configured credential helpers in their isolation
  block. Corrupt, unsupported and legacy plain-text grants need a new GitHub
  sign-in.
- **Settings.** Settings loads and scope connections
  require the requesting page ID; request IDs provide correlation only. Command
  and config operations registration takes consumer-supplied labels, keywords
  and order.
- **Shared windowing (`longhorn-windowing`).**
  `execute_tauri_window_apply_in_place` replaces the removed Tauri dispatch and
  owning apply APIs. Import shared clocks, close handlers, capture records and
  flush ports from `longhorn-windowing`. The Tauri lifecycle services and default
  capture constructors no longer take a mapper; explicit capture and readback
  mapper constructors validate host support. Custom capture backends return
  `TauriWindowCaptureError`. GPUI user-close handlers take `&self`.
- **Agent-control discovery (`longhorn-agent-control`).** `DiscoveryRecord::is_stale`
  and `HOST_SELECTION_ENTRY` are removed; the release names no replacement.
  `DispatchMetrics::mutation_replays`, which was hard-coded to zero, is removed.
- **Desktop support installs (`longhorn-tool-provision`, `longhorn-config`).**
  `CommandRunner` is a struct built with `CommandRunner::default()` or
  `CommandRunner::with_timeout`, and the `From<RunnerError>` conversion into
  `InstallError` is removed. `InstallError` and `InstallFault` report their cause
  through `Error::source`. `encode_locator` returns a `Result` and refuses a
  non-UTF-8 explicit root; `locator_matches` returns the new
  `StorageLocatorError`, which `longhorn-config` maps.
- **Config correctness (`longhorn-config`, `longhorn-tauri-config`).**
  `ConfigStore::load` is strictly read-only: a terminal restore journal now
  needs an explicit `recover_restore`, a coordinated load-set, or a mutation.
  `RecheckCurrent` is distinct from `CaptureRollback` for stale restore targets.
  `StorageTransitionAction::Conflict` is new. `AgeInspectionOutcome::ProviderFailed`
  and `AgeReencryptionError::SourceProviderFailed` separate provider failures
  from `Locked`. `TauriDirectorySnapshot::Linux.runtime_dir` is
  `Option<PathBuf>`. The unused `longhorn-tauri-config` `ports` traits and
  `RestoreUnlockState` are removed.
- **Support log (`longhorn-support-log`).** `LogStats` gains
  `repair_required: bool`, which latches and defaults to `false`. Code that builds
  or exhaustively destructures `LogStats` must handle the field. Contract 025
  matches.
- **Settings search (`@inflatable-cookie/longhorn`, Svelte).**
  `searchSettingsRegistry` takes a third `services: SettingsSearchHostServices`
  argument, and `SettingsSession` requires `searchHostServices` in its options.
  Each Svelte consumer passes a `foldCase` that follows its host's locale; there
  is no default. Rust callers are unchanged.
- **SQLite engine: rusqlite 0.40.2 to 0.41.1.** libsqlite3-sys moves 0.38.2 to
  0.39.1, which clears RUSTSEC-2026-0459 (SQLite CVE-2026-119375). `bundled` and
  `backup` stay on. rusqlite is a dev-dependency of `longhorn-config`, so no
  shipped Longhorn code or Rust signature changes; consumers see the move only
  through the lock. A consumer that depends on rusqlite 0.40.x itself must move
  to 0.41.x in the same update, because libsqlite3-sys declares
  `links = "sqlite3"` and cargo resolves one copy per graph. Tom's decision
  28232532 (2026-11-23) takes this as a Breaking change.

### Added
- **`longhorn-git-workspace`: exact worktree identity.**
  `Git::worktree_identity(&self, root: &Path) -> Result<WorktreeIdentity, GitFailure>`
  reads the worktree and repository identity without redaction: canonical root,
  canonical `--git-common-dir`, Unix device and inode metadata, the exact branch
  short name (or `None` when detached), and the full `HEAD` object ID. It
  compares with `same_repository` and `same_worktree`, and `Debug` formatting
  is opaque. `WorktreeIdentity` and `FilesystemIdentity` are new public types.
  The worktree root and Git common directory are read as exact bytes: the
  runner drops only Git's single trailing record terminator (one `\n`) and
  builds the path with no trim and no lossy decode (`OsStr::from_bytes` on
  Unix). Sibling repositories such as `repository` and `repository ` (or a
  non-UTF-8 name and its lossy twin) therefore no longer resolve to one identity.
  The `HEAD` object ID and the `symbolic-ref` branch keep their existing handling.

### Changed
- **Toolchain and MSRV.** The workspace pins Rust 1.100.0. The declared MSRV is
  1.97.1, including the release floor and compatibility guidance.
- **Effigy.** `[bootstrap].setup` uses the plain selector string
  `"bootstrap:deps"`, which is Effigy v0.15 strict manifest syntax.
- **Tauri adapters (`longhorn-tauri-*`).** Sync `#[tauri::command]` handlers in
  `longhorn-tauri-bridge`, `longhorn-tauri-history`,
  `longhorn-tauri-history-tree`, `longhorn-tauri-licence`,
  `longhorn-tauri-notifications`, `longhorn-tauri-operation`,
  `longhorn-tauri-transfer` and `longhorn-tauri-update` run off the main thread
  through `tauri::async_runtime::spawn_blocking`. Poisoned mutexes and
  `spawn_blocking` join failures report `retryable: false` across the adapters,
  and `longhorn-core::MemoryCredentialStore` returns
  `CredentialError::Unavailable` on poison instead of recovering with
  `into_inner`. `longhorn-tauri-bridge` adds `bridge_plugin`, which installs
  `TauriBridgeState` and wires window-destroyed teardown by default, and
  collapses the duplicate `authorize_*` helpers. `longhorn-tauri-settings`
  accepts an optional `ConsumerSettingsTransactionAuthority` through
  `SettingsHandlerAssembly::with_transaction_authority` and
  `SettingsHostAuthority::authorize_transaction`. `longhorn-tauri-notifications`
  scopes command-triggered change hints to the invoking `WebviewWindow`
  (`window.emit`) rather than the whole app. `longhorn-tauri-transfer` drops its
  shadow client map for `TransferCoordinator::current_client` and releases the
  coordinator mutex before calling the panel and Surface transfer adapters.
- **Tauri and GPUI windowing.** Ordered window apply, dependency skips,
  convergence readback and lifecycle ports are shared. A native window whose
  admission fails is discarded or tracked for cleanup retry. GPUI close flushes
  pending captures, and shutdown receipts expose per-window failures. Tauri
  capture and event translation use the platform desktop mapper. Restore keeps
  the saved home through fallback, and the config sink serialises overlapping
  flushes.
- **Settings registry replacement (`longhorn-poodle-svelte`).** One settings
  registry replacement installs at a time, and each turn installs the newest
  queued generation. A replacement turn belongs to the session that started it.
  The update centre and compact status share the check, stage-or-apply and
  "not now" commands with the sampled projection.
- **History, operation and notification kernels.** `ForkHistory` fork pruning
  and branch-page projection are bounded without full-lineage materialisation.
  `NotificationLedger::publish_once` remembers producer tokens across dismissal
  and pruning in a bounded FIFO ring of up to 4,096 distinct tokens. A no-op
  `change_retention` preserves the revision. Terminal events stay observable
  under zero terminal retention, through post-commit `operation()` accessors on
  receipts and `OperationTeardownOutcome::Completed`. `retry_of` no longer
  requires the source operation to remain retained.
- **Agent-control tool dispatch.** `listen` tracks a per-document epoch across
  page reloads, so events starting at seq 1 are not dropped by a stale cursor.
  It records a full-document load as a `navigation` event at seq 1 and keeps the
  64-event ring. Selection updates go over a `watch` channel without resetting
  the 50 ms page-event poll interval or its 2 s per-eval bound.
  `wait_for` timeouts are bounded
  (`MAX_WAIT_TIMEOUT_MS = 300_000`, advertised in the MCP schema). Selection
  tombstones are bounded at 300 s and 256 entries. Discovery refuses to write
  tokens when `0o600` permissions cannot be enforced. `RecordingDispatcher`
  wakes idle callbacks at their deadline through a pure-`std` timer reactor,
  holds `max_outstanding_calls` occupancy until the callback future finishes or
  drops, and handles unrepresentable durations through `Instant::checked_add`.
  The agent-control shim bundle is regenerated for these source changes.
- **Update installs (`longhorn-update-install`).** Native installs stage per
  target and clean up only the current attempt's files. Unix directory locks
  protect live workspaces from updates to ancestor bundles, and serialise updates
  that share a parent, including privileged replacements. Platforms without
  these locks refuse native bundle replacement. Install targets inside the
  reserved workspace namespace are refused. If publishing and rollback both
  fail, the error reports both faults and the preserved bundle's path. The next
  apply verifies the backup's filesystem identity before restoring it.
  Unverified and legacy backups remain available for manual recovery.
- **Update native content.** Snapshots report `notChecked` before a channel
  check. Superseded or cancelled transfers cannot stage bytes. Tauri emits change
  hints for checks, channel selection and deferrals. Bundle installation rejects
  extra archive roots and uses the executable's location to corroborate package
  provenance. Child-view sources require HTTPS or loopback HTTP. Isolated-window
  requests refused at capacity can retry with the same ID. Replay history and
  teardown reports are bounded. Generation gates return the admitted attachment,
  and equal generations compare successfully.
- **Desktop support installs (`longhorn-tool-provision`).** Managed-tool installs
  publish a raw asset named like its binary. Probes and login-shell runs get no
  standard input and stop after 30 seconds by default. Release listings are read
  to 4 MiB, and a zip's declared entry count is refused before its directory is
  parsed. The install swap renames the verified candidate into place. Tool,
  binary and PATH lookup names follow the managed-name rule (ASCII letters,
  digits, `.`, `_`, `-`, `+`, no leading dot). Browser URL error indices are
  offsets into the whole URL for `http://` and `https://`, and a backslash ends
  the host for `https` as it does for loopback, so a backslash cannot hide
  credentials from the `@` rule. The browser callback survives a
  closed tab after a valid redirect. The real-keychain credential tests run only
  with the `real-keychain-tests` feature. `longhorn_support_log::single_line` is
  public, and the diagnostics bundle uses it.
- **Transfer snapshots.**
  Snapshots probe windows independently and issue epochs without retaining
  destroyed window IDs. Client-change hints are ordered per webview, and each
  renderer subscribes to its own webview.
- **Windowing scheduler.** Lifecycle wake queues stay
  bounded during rescheduling.
- **Bindings (`longhorn-bindings`).** Generated tagged-union output and its
  fixtures cover nested tagged unions.
- **Repository checks and tooling.**
  - `scripts/rust-command.sh` is executable and has direct-invocation tests.
    `proof:artifacts` runs `check:bun-deps` before starting proofs and keeps
    failing member stdout. `fmt:rust` checks prototype workspaces and staged
    example crates through `scripts/fmt-rust.ts`. `check:prototypes` runs
    Clippy with `--all-targets --locked -- -D warnings`. `scripts/msrv.ts` reads
    `[workspace.package].rust-version` as TOML.
  - `qa:docs:changelog` parses a staged minimal copy, so checkout release
    journals do not affect it. The `longhorn-docs` QA group covers
    `changelog.d` fragments, so docs-touching tasks no longer hit
    `needs_planner` on a fragment scope token. Stale `doctor`, greenfield and
    MSRV knowledge facts are updated.
  - Consolidated runner-tool checks and shared proof helpers across artifact
    verifiers. `effigy proof:guide-corpus` runs the guide proof. Bridge
    neutrality markers match whole words.
  - Local private-output Rust routes take their default scratch root from
    Oxide's registered removable build pool (`oxide pool describe --json`).
    `OXIDE_TASK_SCRATCH_ROOT` still wins, and a missing pool or root fails
    closed, naming that variable. The internal-drive refusal is judged by volume,
    not `TMPDIR`. `scripts/external-scratch.sh` runs the same TypeScript rule.
    `scripts/private-cargo.sh --build-dir` and the `buildDir` option of
    `privateCargoCommand` are removed. Build scratch allocation is hardened
    against redirected bases, and Rust lint coverage is widened.
  - TypeScript and Svelte selectors install missing Bun dependencies with the
    frozen lockfile and verify the required markers before continuing. Papercut
    8ae3913b is closed.
  - TypeScript and Svelte QA check strict package and script types, Svelte
    warnings, type-aware lint, formatting and unused code. The published adapter
    has no wildcard Svelte declaration. Rust QA adds a local unused-dependency
    check, and the release flow checks dependency licences, sources and bans
    alongside advisories.
  - `docs/reference/held-surface.md` records every public item a quality pass
    kept, in a checked `Assessed And Retained` table. The `held-surface` gate
    fails when a listed item disappears without its row being removed.
    `AGENTS.md` records that having no callers is not grounds for removal.

### Fixed
- **Licence (`longhorn-licence`).** Check-only symbols (`*~$=U`) are rejected in
  the key body. A `TrustBasis::RemoteAssertion` with no `lease_until` reports
  `Usability::LeaseLapsed`. `TokenRedemptionSource::renew` and `release` return
  `ActivationError::MissingActivationId` when `activation_id` is absent or
  empty. `constant_time_eq` aligns with `longhorn-agent-control`. Contract 019
  documents product binding, no-lease remote assertions and the consumer-integrated
  Contract 018 update-window boundary.
- **Config (`longhorn-config`).** Backup archive inspection enforces real
  inflated-byte and ratio bounds. First writes sync every created ancestor
  directory. Domain-file reads are bounded. `SourceRetained` storage-transition
  recovery cleans partial ordinary target copies.
- **Settings (`longhorn-settings`).** Rust enforces readable scopes and reset
  authorisation. Seal checks that pages can read the scopes of their writable
  units. Config refusals stay typed, revision exhaustion is checked before
  publication, and Rust and TypeScript reject contradictory snapshots. Settings
  scope connections record accepted snapshots after acceptance, not inside the
  predicate. Settings shell apply, guard and reset failures show on the page.
  Config pages show a restore refusal's detail, not its protocol code, with
  archive timestamps localised.
- **Agent-control.** `set_file_input` validation streams without allocating
  decoded buffers, runs once on the MCP-to-Tauri path, and maps to
  `ToolError::InvalidArgument`. Individual `poll_until` checks are bounded.
  Selection expiry runs through one shared-clock sweep with atomic outcome
  delivery. Cancelled and shutdown waits map to `ToolError::CancelledSelection`.
  The stdio carrier disables HTTP proxies and keeps `InstanceToken`'s
  `SecretString` posture.
- **PTY (`longhorn-pty`).** Every OS-backed `Failure` carries `raw_os_error`, and
  `Failure::raw_os_error()` reads it; `Display` shows `(os error <code>)`. `CLOEXEC`
  is set atomically on slave opens, descriptor duplications and master opens on
  Linux, FreeBSD, macOS and illumos. An output reader blocked on a full queue
  wakes when `close()` runs, including after the child has exited, so descriptors
  are released even if the consumer stops reading. Contract 028 matches.
- **Transfer (`longhorn-transfer`, `longhorn-surface-transfer`).** Underlying
  `TransferErrorCode` rejections promote, and `TransferAbort::from_panel`
  preserves `layout_code`. Expired leases are reclaimed before `maximum_leases`
  capacity is enforced. Empty-display target windows are validated against
  `allowed_target_windows` in `SurfaceTransferPolicy::new`. Non-test panics in ID
  conversions, session consumption and post-commit Surface checks become total
  conversions and `HostReconciliationRequired`. `map_mutation_error` matches
  `LayoutMutationRejectionCode` and `SurfaceMutationRejectionCode` exhaustively.
  Host-binding indexing and registered Surface document loading are shared.
  Contract 011 matches.
- **Transport and subscriptions (`@inflatable-cookie/longhorn`,
  `longhorn-poodle-svelte`).** A throwing consumer callback in
  `listenTauriBridgeJob` no longer ends the job or reports a malformed event. A
  malformed progress event during terminal registration releases the terminal
  listener. Renderer subscriptions stay active when a consumer callback throws.
  Malformed events are reported separately. Teardown releases every job listener,
  and a failed port operation keeps its original rejection. Native-content
  connections share the snapshot connection lifecycle: an invalid request no
  longer leaks its pending slot, and `dispose()` finishes cleanup before it rejects
  with an unlisten failure. A start failure, already in `failures()`, still does
  not reject `dispose()`. Shared `CheckedSnapshotConnection` gains `offer()`.
- **Svelte.** The operation panel closes its cancel dialog when the request fails,
  and the failure shows on the page. Window drag ignores tab, link, menu-item,
  label, summary and editable targets. A rejected settings rail navigation takes
  the shell's action-failure path. The failure callout sits above the page region,
  so it survives a session failure, and focus moves only after the route commits.
- **Support log (`longhorn-support-log`).** A failed append truncates the live
  file to its pre-append length, so a later line cannot join a partial JSON
  record. If truncation fails too, the log latches `repair_required` in
  `stats()` until the operator repairs the file. Contract 025 whole-line
  promise holds.
- **Deadlines and loopback limits.** Unrepresentable browser and process
  deadlines are rejected before waiting or spawning. The loopback request-head
  limit is enforced before its terminator is parsed.
- **Bindings (`longhorn-bindings`).** Tagged-union arms are scanned in one
  depth- and string-literal-aware pass, so quoted delimiters and nested tagged
  unions are no longer mistaken for outer tags.
- **Core bridge.** Bridge reconnect and negotiation apply state only after a valid
  receipt is prepared. A forbidden reconnect reports `retryForbidden`. Keymap
  loads reuse compiled state and expose stable unavailable and validation codes.
  Bridge and command host errors have generated TypeScript bindings. Restore
  identity labels have shared Rust and TypeScript parity coverage.
- **Shared windowing scheduler.** Accepted wakes are preserved when timer startup
  fails.
- **Git workspace device flow (`longhorn-git-workspace`).** A `slow_down` answer
  only lengthens the poll interval: the server's suggestion is adopted when it is
  longer than the interval in force, and the 5 s step applies otherwise, so a
  shorter suggestion can no longer shorten the wait.
- **Tauri operation adapter (`longhorn-tauri-operation`).** A poisoned executor
  derives its dispatch code from
  `OperationHostErrorCode::ExecutorStateUnavailable::as_str()` with
  `retryable: false`, instead of a string literal with `retryable: true`.
- **Settings search (`@inflatable-cookie/longhorn`,
  `longhorn-poodle-svelte`).** Case folding follows the host on both routes. The
  Svelte route lowercased with a fixed en-US policy, so Turkish dotted and
  dotless `i` could disagree with Rust.

### Security
- **Git workspace credentials (`longhorn-git-workspace`).** Credentials stay
  scoped to the resolved HTTPS destination. Git skips ambient credential helpers
  even when the app supplies no credential. Askpass credentials own their helper.
  The migration steps are under Breaking.
- **Clear RustSec advisories in the lock (RUSTSEC-2026-0457, -0459, -0461, -0462, -0486, -0592, -0594).** `rmcp` moves to 3.9.0 (used by `longhorn-agent-control`), `cookie` to 0.18.3, `rustix` to 1.1.6, `url` to 2.5.9, and `libsqlite3-sys` to 0.39.1 through rusqlite 0.41.1 (see the Breaking entry above). The `rmcp`, `cookie`, `rustix` and `url` moves are lock-only, with no manifest change, and the affected prototype locks move the same way. The `chrono` unmaintained advisory (RUSTSEC-2026-0543) has an allowance because no safe upgrade exists. See `docs/knowledge/contracts/release.md` "Lock maintenance".

## [0.5.0] - 2026-11-02

### Breaking
- **`longhorn-tauri-native-content-child-view`: `ChildViewSpec::new` takes a
  `ChildViewSource` instead of a bare `Url`, and `source()` returns the typed
  carrier.** Remote callers migrate the one constructor argument:
  `ChildViewSource::external(url)` keeps the historical behavior exactly —
  HTTP and HTTPS with a host, re-checked at construction, every other scheme
  still refused with `ChildViewError::InvalidContentSource` — and
  `ChildViewSource::bundled(path)` names a bundled local document (next
  entry). New public types `ChildViewSource` and `BundledLocalDocument`; new
  `ChildViewError::InvalidBundledContentSource` (wire code
  `child:invalid-bundled-source`). This is the release's one coordinated
  pre-1.0 source break; contract 017 owns the carrier and its guarantees.

### Added
- **`longhorn-tauri-native-content-child-view`: a bounded bundled local
  document source.** `ChildViewSource::bundled` carries one validated relative
  path inside the application bundle: at most 512 UTF-8 bytes of ASCII
  alphanumerics plus `-`, `_`, `.`, `/`, with no absolute, empty, `.` or `..`
  component, no backslash, and no URL syntax, so the path can neither escape
  the bundled asset root nor inject a scheme. At attach the runtime maps it to
  the application's own asset protocol (Tauri `WebviewUrl::App`): no local
  server, no arbitrary filesystem URL, and no other scheme ever enters the
  adapter. The concrete app-scheme URL is Tauri-owned, observable through
  `ChildViewAdapter::current_url`, and never promised by name. The host
  application keeps bundled-document identity, path authority, CSP, capability
  scope, and injected-script policy; a bundled child deliberately shares the
  app's local capability scope, remote capabilities stay excluded unless the
  consumer grants them, and the consumer navigation policy remains the sole
  admission path for every navigation, including the initial document's
  origin. Evidence: `proof:native-content:local` runs a real macOS WKWebView
  from a staged product-free consumer through the public API, proving observed
  asset-protocol source identity, scoped IPC with an initialization-script
  nonce round-trip through exactly one permitted command, an unpermitted
  command denied by capability scope, the page's explicit CSP, denied popup
  and navigation escapes with the adapter-typed denial, and truthful detach,
  post-detach, and idempotent-teardown receipts. Limits: Windows and Linux
  are unproved, hosts without a GUI session cannot run the proof, and the
  evidence carries no signed-product or data-store isolation claim beyond the
  app-owned policy.

### Changed
- **Local Cargo routes run through Oxide.** `scripts/rust-command.sh` is the
  one switch point: locally every ordinary Cargo command runs as `oxide` (or
  `oxide exec` for a leased tool) and build output stays on Oxide-managed
  roots, with private directories refused on the internal drive, the account
  home, and volume roots; with `GITHUB_ACTIONS=true` the same routes keep
  plain Cargo, so hosted CI is unchanged. The published crates and packages
  are unchanged.

### Security
- **Move the pinned `age` line from 0.12.1 to 0.11.5 (RUSTSEC-2026-0387).**
  crypto-bigint 0.5.5, reached only through age 0.12.1's mandatory
  hpke → p256 → elliptic-curve path (present even with
  `default-features = false`), has constant-time execution issues patched
  only in crypto-bigint ≥ 0.7.6, and no published 0.12.x release drops the
  path. age 0.11.5 carries no mandatory p256 or hpke dependency at all, and
  `longhorn-config-age` uses only the X25519 and scrypt recipients both
  lines implement, so the public API is unchanged. The compatibility
  contract is the age v1 file format (contract 004), not a crate line:
  retained ciphertext captured through the public API on 0.12.1 — the line
  0.4.3 shipped — still restores (fixtures/config-age-compat;
  `test:config-age:security` decrypts, inspects, rotates and retains it for
  the X25519 recipient and human passphrase envelopes, wrong keys stay
  `Locked`, tampering still reports `Corrupt`). `check:config-age:security`
  proves a public Git-dependency consumer at the exact tree resolves age
  0.11.5 with no vulnerable chain and compiles, that a consumer lock
  resolved on the old line is refused under `--locked` and one re-lock lands
  clean, and the release advisories gate runs green with no new `deny.toml`
  allowance. The lock delta removes twenty packages (the whole hpke chain,
  including crypto-bigint, ml-kem and sha3) and moves only age, age-core,
  bech32, nom and the i18n-embed/fluent line age 0.11 requires; no prototype
  lock carries age, so no prototype lock moves. The downgrade first resolved
  i18n-embed-fl 0.9.4, whose proc-macro-error2 dependency is unmaintained
  with no patched release (RUSTSEC-2026-0173); rather than adding an
  allowance, the lock pins i18n-embed-fl 0.9.1, the newest `^0.9` release
  that still uses the proc-macro-error 1.0.4 the existing RUSTSEC-2024-0370
  allowance already covers through the gtk build chain, so the advisories
  gate is green with no new ignore.

## [0.4.3] - 2026-10-23

### Fixed
- **`longhorn-pty`: close and resize no longer report `Failure::Signal` for a process group that has already exited.** macOS and BSDs answer `EPERM`, not `ESRCH`, when a group holds only zombies, and close treated that as a live group that refused the signal. An exited leader is read with `waitid` `WNOWAIT`, so the zombie is not collected during close. A live leader that refuses is still `Signal`. Contract 028 records the rule and its one limit.

### Security
- **Update `tokio` to 1.53.3 (RUSTSEC-2026-0386).** `AsyncFd`'s safe constructors accepted any `AsRawFd`, so safe code could register an arbitrary or closed file descriptor; when the number was reused, dropping the first `AsyncFd` deregistered the reused number and left the original registration in `epoll`, which a readiness event could turn into a use-after-free. This is a lock-only update to a fixed release in the root lock and the six affected prototype locks; the manifests' `1.45.1` requirements and 1.53.3's `rust-version` 1.71 already admit it, and there is no `deny.toml` allowance.

## [0.4.2] - 2026-10-23

### Added
- **`longhorn-git-workspace`: the generic Git workspace mechanism (contract 029, part 5a).** One `Git` handle carries the application's `GitPolicy` and its `longhorn-redact` `Redactor`. It runs `git` as a fixed program with an argument vector, applies the application's isolation before the invocation credential, asks for a credential only for `fetch`, `push` or `ls-remote` on an HTTPS GitHub remote, retries once with a refreshed credential after an authentication refusal, and bounds both captured streams at 64 KiB. Readings cover worktree status, local and remote branch discovery, the upstream and detached state, a read-only remote preflight that classifies five refusal readings, Legacy Sync (clean, upstream-only, `--ff-only`, ahead-only push), an explicit branch switch that treats untracked content as in-progress work, clone into an invocation-owned staging folder with a same-parent exclusive rename, and attach over an explicit expected branch and remote. Successful standard output, `origin_url` and an attach reading's observed remote are redacted with the caller's redactor and the invocation secret before they leave the crate; failure details are Git's own words, run through the same redaction and cut to a 4 KiB / 40-line tail, and failures carry machine facts and wire codes, never product copy. Standard error is captured from its head within 1 MiB and redacted before any cut, so a truncation cannot expose a private-key body whose opening line was removed. A hard wall clock or a clone cancellation kills the child's whole process group on Unix and the wall clock covers the output drain, so a descendant holding a pipe cannot stretch the bound even after Git exits; the synchronous policy hooks run outside that bound, and a hook that overruns makes the call report a timeout as soon as it answers. The tests run real `git` against temporary local repositories and bare remotes with no network. GitHub sign-in is part 5b. Contract 029 owns the API, the guarantees and the deltas from Acowtancy Desktop's `git/mechanism/`.
- **`longhorn-git-workspace`: GitHub device-flow sign-in, grant refresh, commit identity and the askpass credential bridge (contract 029, part 5b).** `run_device_flow` runs GitHub's device and poll protocol against explicit endpoints: a one-time code and GitHub's pinned verification page in progress, a five-second poll floor, a `slow_down` increase, a wait bounded by the shorter of the declared expiry and fifteen minutes, and the caller's cancellation flag. One grant is stored in the caller's contract 021 credential slot as a version 1 JSON document with whole-second expiries; `fresh_access_token` and `force_refresh_token` refresh inside a five-minute skew under one process-wide lock, so concurrent callers cause one exchange, a rotated grant is reused after a rejection, a refused refresh token reports `SignInExpired` while leaving the store untouched, and a refresh writes only over the grant it read, so a concurrent sign-out stays signed out and a newer sign-in is adopted rather than overwritten. `GithubCommitIdentityResolver` derives the operator's name and GitHub noreply address from the profile, cached per account and per token digest. `AskpassConfig` names the token variable and `GitCredentialBridge` writes this crate's own askpass helper into an owner-only temporary folder, so the token reaches Git only as one child environment value, never in an argument, file, log line, error or `Debug` output, and the folder is removed when the bridge is dropped. Tokens use `secrecy`, every store and slot is injected, production endpoints are `https`-only, and the loopback fixture server that proves the flow is behind the off-by-default `fixture-server` feature. Contract 029 owns the behaviour, the failure and progress vocabulary, and the deltas from Acowtancy Desktop's `git/mechanism/auth.rs`.

### Changed
- **Split-shell is retired; its proof compositions are renamed to `minimal-shell`.** The app-shell, settings-composition and bridge-topology proofs keep every assertion under the neutral name. No published API changes.

### Security
- **Update `ctutils` to 0.4.5 (RUSTSEC-2026-0385).** Earlier versions could compile `Choice` mask generation or `u128` comparisons into branches on secret values on some platforms. This is a lock-only update to a fixed release; no `deny.toml` allowance.

## [0.4.1] - 2026-10-22

### Added
- **`longhorn-storage-layout`: resolve one root and read one native directory fact on demand.** `resolve_storage_root` shares the complete resolver's per-root rules and accepts partial platform facts; `native_platform_directory_fact` reads only the environment entries for the requested fact. Complete layout resolution and its digest are unchanged. Contract 004 owns the behavior.

## [0.4.0] - 2026-10-21

### Breaking
- **Storage identity, layout resolution, `StorageRoots`, and the bootstrap locator moved from `longhorn-config` to a new `longhorn-storage-layout` crate.** The moved surface is `StorageIdentity` (with its error types), `resolve_storage_layout`, `StorageLayoutRequest`, `StorageLayoutOverrides`, `ResolvedStorageLayout`, `ResolvedStorageRoot`, `StorageLayoutDiagnostic`, `StorageLayoutError`, `StorageProfile`, `StorageProfileIdError`, `StorageLayoutWarning`, `StorageLeafProvenance`, `StorageRootProvenance`, `PlatformDirectoryFact(s)`, `TargetPlatform`, `RootKind`, `AccessMode`, `StorageRoots`, `StorageRootError`, `root_kind_id`, `resolve_storage_bootstrap_paths`, `inspect_storage_bootstrap`, `StorageBootstrapState`, `StorageBootstrapSelection`, `StorageBootstrapPaths`, `StorageBootstrapOrigin`, `StorageBootstrapRecovery(Kind)`, `StorageProfileLocator`, `StorageProfileSelection(Error)`, and `encode_locator`/`locator_matches`. Imports change from `longhorn_config::…` to `longhorn_storage_layout::…` with no re-export shim (decision 6f239e7b); `longhorn-config` signatures may still take the moved types. The layout digest type is now `StorageLayoutDigest` (same validated lowercase-hex SHA-256 content; `longhorn-config` converts at the transition boundary). `StorageRoots::resolve(&DomainDescriptor)` stays in `longhorn-config` as the `StorageRootsResolve` extension trait. Paths on disk do not change: profiles, profile ids, leaf rules, and the fixed locator path are identical.

### Added
- **`longhorn-config`: read-only human-written TOML domains (contract 004, phase 1).** A domain can declare its file a human-written TOML document, opt-in per domain behind the new `toml` feature (`DomainDescriptor::new_toml`, `.toml` file paths). Longhorn reads the file, maps it to the domain's JSON value model, validates it, and migrates it in memory; the file holds the value only, with the reserved optional `schema_version` key (missing means current). Parse, schema-version and type-mapping refusals are typed recovery states with a one-based line and column. Every write path — mutation, checked mutation, debounced flush, destructive migration rewrite, and restore — refuses with a typed error naming the domain as human-written and read-only, and the file is never touched. Backups record the domain as an explicit exclusion (`human-written, read-only in phase 1`); an explicit catalogue include is refused. Off by default: without the feature no TOML code or dependency compiles and JSON domains are byte-identical. The new `test:config-toml` selector runs the feature-gated suite and joins `qa`.
- **`longhorn-storage-layout`: storage identity, the pure layout resolver, `StorageRoots`, and the fixed locator without the config store.** Depends on `std`, `serde`, `serde_json` (the moved bootstrap locator's document), and `sha2`, so a CLI resolves Longhorn's storage paths without the store, backup, or Tauri stacks.
- **`longhorn-storage-layout`: a native platform-directory facts provider for hosts without Tauri.** `native_platform_directory_facts` reads the environment once; the pure `native_platform_directory_facts_from` takes an injectable snapshot. macOS derives from `HOME`, Linux applies the XDG spec defaults (`XDG_RUNTIME_DIR` has none and is never guessed), Windows uses `LOCALAPPDATA` and roaming `APPDATA`. A missing or non-absolute source reports its facts unavailable; nothing is invented. A parity test proves the provider and `longhorn-tauri-config::platform_directory_facts` produce identical facts for the same machine snapshot on macOS, Linux, and Windows.

## [0.3.4] - 2026-10-21

### Added
- **`longhorn-pty`: bounded pseudo-terminal sessions.** Runs one caller-selected child with an exact caller environment, streams raw output in 8 KiB chunks through an eight-event bounded queue, writes input, resizes the controlling terminal, and closes the process group with a 300 ms `SIGHUP` grace before `SIGKILL`. Child exit is reaped once and reported as a typed code or signal; post-exit operations are `Ended`. Contract 028 owns the API and compatibility deltas from Acowtancy Desktop's `pty.rs`.
- **`longhorn-tool-provision`: synchronous managed-tool install, health and login-shell PATH (part 4c).** `install_tool` composes release resolution and bounded download with the verified-artifact gate, extraction, optional vendor-team check, a scripted version probe, floor and pin rules, per-binary concurrency refusal, an atomic replace with rollback, and an atomic JSON receipt. A host-injected runner starts fixed programs with argument vectors; probe output passes through `longhorn-redact` and `longhorn-support-log` before errors retain it. `tool_health` reports installed, absent or broken state. `LoginPathCache` reads neutral markers from the fixed login-shell script, caches the PATH, and supports refresh with a process-PATH fallback. Catalogue entries, install roots, user bin paths and user-facing stage labels stay with the consumer. Contract 027 owns the protocol and compatibility deltas.
- **`longhorn-tool-provision`: release resolution and bounded download for managed tools (part 4b).** A `ReleaseSource` resolves a repository's release (a pinned tag exactly, or `latest`) and downloads one listed asset. Drafts, pre-releases, a version floor and a tag mismatch are refused; the listing's rate limit is named and never paged around. The API token is a `SecretString` sent only on listing calls to the configured origin, never on a download and never on a cross-origin redirect. The download counts the bytes the server sent against a caller bound, removes a partial file on any failure, and never overwrites. The HTTP client is blocking, with rustls. Test support is the `fixture-server` feature: a loopback server, off by default. The catalogue, pins, token broker and asset names stay with the caller. Contract 027 owns the behaviour and lists the deltas from Acowtancy Desktop's `releases.rs`.
- **`longhorn-tool-provision`: verified artifacts and bounded extraction for managed tools (part 4a).** A declared digest (release checksum file, `.sha256` sidecar or pin) is checked before any extraction; a `VerifiedArtifact` is the only input extraction takes. One requested member is written into a caller's staging directory, within a member and real-decompressed-byte bound, with the mode set here. A declared vendor team is checked on the staged executable through an injected signature port, whose macOS adapter runs the fixed `/usr/bin/codesign`. Failures are typed as refusals or host faults. Contract 027 owns the behaviour; release source and the install pipeline follow in parts 4b and 4c.
- **`longhorn-archive-path`: the shared archive path rule.** The entry-name and link-target rule that `longhorn-update-install` used privately, moved here unchanged so the managed-tool extraction shares it. `longhorn-update-install`'s behaviour is unchanged.

### Fixed
- **The agent-control stdio carrier now gives the router's own replies.** The carrier stamped every post-`initialize` POST with the protocol version from the server's `initialize` answer, but rmcp always answers that handshake with the newest legacy version it supports (`2025-11-25`), even when the harness asked for `2026-07-28`. The server therefore treated the carrier as a legacy peer: `tools/list` and `resources/list` lost the `resultType`, `cacheScope` and `ttlMs` fields the direct router returns (all three come from rmcp's own `ListToolsResult`, SEP-2322/SEP-2549), and `subscriptions/listen` was refused as method-not-found. The carrier now stamps each POST with the harness's own declared version — its per-request `_meta` first, then its `initialize` params — and keeps the handshake answer only as the fallback, so both paths are answered from the same source and cannot drift. The widened `test:agent-control-carrier` selector runs the whole carrier suite with `--features client`, joins the `qa` aggregate, and is selected by the new `agent-control-carrier` QA group.
- **`longhorn-pty` keeps one slave descriptor until the reader's final drain.** Some platforms discard unread slave-to-master pseudo-terminal data on last slave close. The crate dropped every parent slave copy at spawn, so a child that wrote and exited before the reader's next 10 ms drain lost its output (observed as an empty result with a clean exit under `cargo test` load). The parent now retains one slave descriptor, moved to the output-reader thread and closed after its final post-exit drain, so last close cannot precede delivery. A zero-byte master read now polls again instead of ending the stream, since it carries no information about the child. Contract 028 records the retained descriptor.
- **`evaluate` separates the origin marker from the caller's JavaScript.** Scripts beginning with `(` or `[` now run and retain their synchronous completion value.
- **The release dry run's changelog check passes with the released Effigy 0.14.1.** Effigy 0.14.1, the version `release.yml` installs through `setup-effigy`, refused the four unindented continuation lines of the `[0.3.1]` `Changed` toolchain entry ("unexpected content in category"); the local build accepts them, so the runner's `test:release-tooling` verdict could not agree with the local one. Those continuation lines are now indented, with no wording change. `scripts/check-changelog.ts` takes an `EFFIGY_BIN` override so both builds can be checked, and a failing test verdict prints Effigy's reason instead of only `fail`.

### Security
- **Allow RUSTSEC-2026-0374 (`basic-toml` unmaintained).** It is reached at build time through `age` -> `i18n-embed-fl`/`i18n-embed-impl` -> `i18n-config` -> `basic-toml`, only via proc macros. It is not a vulnerability, no upgrade drops it, and it is build-time only. Accepted 2026-10-20.
- **Update `rustls-webpki` to 0.103.16 and `webpki-root-certs` to 1.0.10 (RUSTSEC-2026-0382, RUSTSEC-2026-0383).** `rustls-webpki` 0.103.15 could accept certificate paths through the distrusted emSign roots (RUSTSEC-2026-0382), and `webpki-root-certs` 1.0.9 bundled `emSign ECC Root CA - G3` (RUSTSEC-2026-0383). Both are lock-only updates to the fixed patch releases; no `deny.toml` allowance. `cargo deny check advisories` passes again.

## [0.3.3] - 2026-10-19

### Added
- **`longhorn-diagnostics`: the pure diagnostics bundle renderer.** Takes the facts a desktop app has already gathered, lays them out as one support text in sections, and redacts the whole text last with the app's `longhorn-redact` `Redactor`, so a span that crosses a label and its value is still caught. Returns the redacted text and how many `[redacted]` markers it holds. Interior CR and LF are escaped before the pass, so a seed whose match depends on a raw line break is not caught; contract 026 lists that delta and pins it. No filesystem, network, credential or app-state reads, and no product text: the title, redaction statement, section list, headings, labels and empty-section sentences are caller values. Contract 026 owns the layout and lists the deltas from Acowtancy Desktop's `bundle.rs`.

## [0.3.2] - 2026-10-19

### Added
- **`longhorn-support-log`: bounded, redacted support logs.** A fixed-capacity `RingLog`, a size-bounded `RotatingLog` with numbered rotations and text or JSON lines, and `bound_field` and `bounded_tail` helpers. Every text goes through the caller's `longhorn-redact` `Redactor` before any cut, so a tail cut never keeps part of a private key. Bounds hold on every write, across threads and restarts; a line over the file bound is dropped and counted. Timestamps are UTC with milliseconds. The app supplies every capacity, bound, directory and file name. Contract 025 owns the behaviour and lists the deltas from Acowtancy Desktop's three logs.

## [0.3.1] - 2026-10-18

### Added
- **`longhorn-redact`: one secret-redaction engine for desktop support text.** A `Redactor` seeded with exact secrets and a `RedactionRules` set (token prefixes, credential assignments, `Bearer`, URL userinfo, PEM private keys, JWTs) replaces each merged span with `[redacted]` and reports `count` and `replaced`. Std only. `RedactionRules::common()` carries published vendor formats; products choose seeds, rules and where output is redacted. Contract 024 owns the behaviour and lists the deltas from Acowtancy Desktop's three redactors.

### Changed
- **`longhorn-poodle` now requires Poodle `v0.4.12`.** Consumers that also
  take Poodle crates by tag must move to the same tag.
- **Update and licence commands return their outcomes.** Awaiting a command
  reports a commit, an update deferral, an authority rejection, or a transport
  failure directly. `lastRejection` clears when the next command starts, so a
  transport failure cannot leave an earlier refusal looking current.
- **The stable Rust toolchain is pinned to 1.97.1.** `rust-toolchain.toml` is
  the one declaration; `ci.yml` and `release.yml` install exactly that channel
  instead of floating `stable`. Rust 1.98 added a Clippy lint that was green on
  1.97.1 locally and red on the runner, and stopped a dry run before publish. The
  MSRV floor (`LONGHORN_GENERAL_MSRV`) is unchanged.

### Security
- **`zip` moves from 8.6 to 9.0.5 (RUSTSEC-2026-0367).** zip before 9.0.0 keeps SUID/SGID bits when it extracts. Longhorn never extracts: `longhorn-config` reads backups in memory. Backup archives stay byte-identical, with no fixture change. The range stays caret, so consumers taking Longhorn by git tag resolve a compatible zip 9.x.
- **`deny.toml` allows RUSTSEC-2026-0370, RUSTSEC-2026-0371 and RUSTSEC-2026-0372 (`zvariant`, `zbus_names` and `zvariant_derive` unmaintained).** They are reached only through `keyring` -> `zbus` 5 on the Linux secret-service path of `longhorn-credential-keyring`. Upstream merged them into zbus 6, which `keyring` 4.2.0 does not use yet. They are not vulnerabilities, and no upgrade removes them.

## [0.3.0] - 2026-09-27

### Breaking
- **`longhorn-poodle-svelte`'s Svelte peer is `>=5.56.8 <6`.** The adapter matches Poodle's own Svelte peer, so consumers can take newer Svelte 5 releases with both packages. This drops Svelte below `5.56.8`; `0.2.2` and earlier declare `>=5.38.6 <=5.56.8`. `proof:pack-typecheck` typechecks the packed adapter at both `5.56.8` and the newest release the range admits. Contract 012 amended 2026-09-26.
- **`longhorn-tauri-agent-control` has no `dev` feature.** The 0.2.0 migration forward is gone. Name `agent-control-evaluate` for what `dev` enabled, or `agent-control` for the surface without `evaluate`. A packaged `agent-control` build is unchanged.

### Changed
- **Every dependency is on its newest compatible release ahead of 0.3.0.** One sweep moved the Rust workspace lock, the eight prototype locks, and the npm lock: Tauri 2.12, wry 0.57, rmcp 3.4 (which renames `ServerInfo` to `ServerConfig` in the agent-control MCP layer), and the rest of their graphs. The exact `rusqlite` pin moves to `0.40.2` with the storage conformance suites green, and the exact Poodle pin moves to `0.4.4` — the workspace dev pin and the Rust git tag together. Poodle's peers are unchanged.
- **`longhorn-poodle-svelte` peers on Poodle `>=0.4.4 <0.5` instead of exact `0.4.4`.** The peers stay optional. A consumer on any Poodle `0.4.x` from `0.4.4` keeps one Poodle copy, and a Poodle patch no longer forces a Longhorn release. Longhorn's own dev and proof installs stay exact `0.4.4`, and `proof:pack-typecheck` typechecks the packed adapter at both ends of the range. Contract 012 amended 2026-09-27.
- **`rusqlite`, `zip` and `age` are caret ranges, not `=` pins.** Consumers taking Longhorn crates by git tag can resolve a compatible newer release, including the SQLite one application graph can hold. Longhorn's `Cargo.lock` stays the tested set; `ts-rs` stays exact `=12.0.1`. Contract 012 amended 2026-09-27.

### Fixed
- **`qa` fails when a release gate needs a tool `release.yml` does not install.** `check:runner-tools` derives the cargo plugins `[release.gates]` requires and walks the release job's steps: each must be installed unconditionally, in the same job, before `effigy release:gates`. An install in a sibling job or after the gates does not count, and a workflow comment naming the tool is not an install. 0.2.2's dry run died on a missing `cargo deny`.

## [0.2.2] - 2026-09-26

### Added
- **Rust-side picker calls join the pending-selection registry.** The consumer carries the page origin into its own command; `begin_host_selection` begins a pending selection for an agent or tells the caller to use the dialog plugin for a human. A missing or malformed origin is human. Result shapes, cardinality, expiry, rejection-as-cancel, and the no-native-fallback rule match the JS route. Contract 022 amended 2026-09-26.
- **Agents can fill an HTML file input with inline bytes.** `set_file_input` names the input and supplies each file's name, optional media type, and base64 content. No path argument, no filesystem read, no OS panel. `multiple` and `accept` are checked before delivery; the decoded total is capped at 8 MiB. Packaged `agent-control` answers it without `agent-control-evaluate`. Contract 022 amended 2026-09-26.
- **One-command version bump.** `effigy release:bump -- <semver>` updates the workspace version and internal pins, the three npm versions and adapter peers, the skill stamp, prototype locks, and the API reference, and promotes `[Unreleased]`. Proofs and boundary tests read `workspace.package.version`.

### Fixed
- **The `set_file_input` 8 MiB cap holds for every caller.** Direct `ControlHandler` callers get the same size and payload refusals as the MCP edge, before any webview work.
- **The Tauri agent-control shim cannot drift from its TypeScript source.** `check:agent-control-shim` rebuilds the webview IIFE and checks the committed asset byte for byte.
- **`effigy release:bump` reports every file it changes and never leaves a partial bump.** `changed` includes lock rewrites. A package.json version line that is not the expected shape is a stop. Planned writes snapshot first; a later lock-sync or API-regen failure restores the tracked files.

## [0.2.1] - 2026-09-25

### Added
- **Agent-originated Tauri JS `open`/`save` can be answered over MCP.** A pending-selection registry publishes `longhorn://agent-control/selection`; `answer_selection` / `reject_selection` settle the waiter. Consumers replace plugin-dialog at call sites with `bindFileSelection`. Human pickers keep the plugin even while the control server is running. `save` chooses a target; Longhorn never writes it.

## [0.2.0] - 2026-09-22

### Added
- **A stdio carrier fronts the contract 022 server for harnesses that cannot
  consume a streamable-HTTP MCP entry.** `longhorn-agent-control-client`
  discovers the live instance through the discovery file, connects to its
  loopback endpoint with the per-instance bearer, and fronts it over stdio —
  one server, one registered catalogue, one policy, one dispatch path. It holds
  no catalogue and exposes no listener, and it is opt-in like `agent-control`.
  `subscriptions/listen` and the `longhorn://agent-control/...` resources are
  typed `Unsupported` over stdio with an explicit reopen condition.

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
- **Update authorization is a held exclusive admission lease.** `UpdateGate`
  acquires a lease from a host-supplied `AdmissionAuthority`, and
  `InstallAuthorization` carries it: the held variant replaces the old
  point-in-time `Approved` (`held` on the wire, was `approved`). The controller
  keeps the lease alive across `apply` and drops it only once replacement has
  returned, so no new conflicting work starts during the swap. A lease the host
  will not grant is an ordinary deferral whose `WorkInFlight` detail is the
  host's own reason — never a failure and never an install. Quiescence stays
  the precondition; the lease is the barrier, and Longhorn still learns no
  application operation. Contract 018 amended 2026-09-22.

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
  graphs. At `0.3.0`, the tree produced three TypeScript packages and 49 Rust crates:
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
