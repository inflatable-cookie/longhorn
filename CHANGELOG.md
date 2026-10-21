# Changelog

All notable Longhorn changes are recorded here. A release tags the repository
and publishes the three TypeScript packages to npm; consumers take the Rust
crates by git tag and the packages by version.

## [Unreleased]

### Breaking
- **Storage identity, layout resolution, `StorageRoots`, and the bootstrap locator moved from `longhorn-config` to a new `longhorn-storage-layout` crate.** The moved surface is `StorageIdentity` (with its error types), `resolve_storage_layout`, `StorageLayoutRequest`, `StorageLayoutOverrides`, `ResolvedStorageLayout`, `ResolvedStorageRoot`, `StorageLayoutDiagnostic`, `StorageLayoutError`, `StorageProfile`, `StorageProfileIdError`, `StorageLayoutWarning`, `StorageLeafProvenance`, `StorageRootProvenance`, `PlatformDirectoryFact(s)`, `TargetPlatform`, `RootKind`, `AccessMode`, `StorageRoots`, `StorageRootError`, `root_kind_id`, `resolve_storage_bootstrap_paths`, `inspect_storage_bootstrap`, `StorageBootstrapState`, `StorageBootstrapSelection`, `StorageBootstrapPaths`, `StorageBootstrapOrigin`, `StorageBootstrapRecovery(Kind)`, `StorageProfileLocator`, `StorageProfileSelection(Error)`, and `encode_locator`/`locator_matches`. Imports change from `longhorn_config::…` to `longhorn_storage_layout::…` with no re-export shim (decision 6f239e7b); `longhorn-config` signatures may still take the moved types. The layout digest type is now `StorageLayoutDigest` (same validated lowercase-hex SHA-256 content; `longhorn-config` converts at the transition boundary). `StorageRoots::resolve(&DomainDescriptor)` stays in `longhorn-config` as the `StorageRootsResolve` extension trait. Paths on disk do not change: profiles, profile ids, leaf rules, and the fixed locator path are identical.

### Added
- **`longhorn-config`: read-only human-written TOML domains (contract 004, phase 1).** A domain can declare its file a human-written TOML document, opt-in per domain behind the new `toml` feature (`DomainDescriptor::new_toml`, `.toml` file paths). Longhorn reads the file, maps it to the domain's JSON value model, validates it, and migrates it in memory; the file holds the value only, with the reserved optional `schema_version` key (missing means current). Parse, schema-version and type-mapping refusals are typed recovery states with a one-based line and column. Every write path — mutation, checked mutation, debounced flush, destructive migration rewrite, and restore — refuses with a typed error naming the domain as human-written and read-only, and the file is never touched. Backups record the domain as an explicit exclusion (`human-written, read-only in phase 1`); an explicit catalogue include is refused. Off by default: without the feature no TOML code or dependency compiles and JSON domains are byte-identical. The new `test:config-toml` selector runs the feature-gated suite and joins `qa`.
- **`longhorn-storage-layout`: storage identity, the pure layout resolver, `StorageRoots`, and the fixed locator without the config store.** Depends on `std`, `serde`, and `sha2` only, so a CLI resolves Longhorn's storage paths without the store, backup, or Tauri stacks.
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
