# Held Surface

Status: checked
Updated: 2026-11-24

Some Longhorn surface is built, tested, and contract-documented but not yet
ready for consumer implementation. This register is the single source of
truth for what is held and why. "Held" never means deleted or half-wired:
the code stays, the docs say it stays, and the trigger below says what moves
it out of this register.

Longhorn is a reusable library. Surface that a quality pass judged and kept
is recorded in [Assessed And Retained](#assessed-and-retained). Those rows are
decisions, not suggestions: having no caller today is never evidence that an
item should go.

Statuses:

- `held-for-consumer` — built against the contract, awaiting a named
  consumer to implement against.
- `planning-evidence` — proved, but no consumer is named yet.

Rules enforced by `scripts/verify-held-surface.ts`:

1. Every entry names a crate or package that exists and appears in the
   [generated API surface](api-surface.md).
2. Held entries are never presented as selectable in the
   [adoption guides](../guides/README.md); the guides link here instead.
3. Awaits and Trigger name a consumer need or product choice, never a task
   or card ID.
4. Moving a surface to selectable is a doc + gate change in one batch.
5. Every retained row names a crate or package from the generated API
   surface, or an example directory for example evidence, and every named
   item is still in that owner's source. A row outlives the item it records.
6. A quality pass must not propose removing an item listed in this file
   unless its listed evidence has changed. Having no callers is not evidence.
7. Items are removed only when they are wrong, misleading, duplicate, an
   alias, constant by construction, or never constructed.
8. A PR that decides to keep or retire such an item updates this table in
   the same PR.

## Register

| Surface | Status | Awaits | Trigger |
| --- | --- | --- | --- |
| age encryption (`longhorn-config-age`, no `bindings` feature) | held-for-consumer | a consumer that needs encrypted backups | contract 004 already documents optional age v1; integration seam = `ConfigStore` capture/restore hooks plus archive inspection. Until then, composition is the documented two-call flow: encrypt via config-age, decrypt and inspect via config-age, then hand plaintext to `longhorn-config`. |
| bridge contract machinery (`longhorn-bridge`: lifecycle machine, replay ledger, ordering, supervision, jobs) | held-for-consumer | a bridge consumer with a real service topology | the optional-server shape needs supervision or ordered delivery. Until then the surface is exercised by the crate's own contract tests; supervision is a stub on both sides of the renderer boundary. |
| layout bounded replay (`BoundedLayoutReplayStore`, `apply_with_replay`) | planning-evidence | a host that needs replay at the mutation boundary | tested since the layout-suite port (2026-08-15); no production caller exists. Contract 002's absorbed layout section carries the mechanism. |
| `MilestoneRetention` (backup retention age buckets) | planning-evidence | a retention product choice | contract 004 documents count/age tiers; milestone buckets are uncontracted. Contract or retire with the retention consumer. |

Application update and licensing graduated from this register on 2026-08-15,
when their crates, renderer surfaces, Tauri hosts and packaged proofs landed.
Both are ordinary selectable systems.

The fork tree (`longhorn-history-tree`, `longhorn-tauri-history-tree`, and
`@inflatable-cookie/longhorn/history-tree`) graduated on 2026-09-28. It is an
ordinary selectable optional system on linear history; see
[contract 008](../knowledge/contracts/008-history-kernel-boundary.md).

## Assessed And Retained

The config crate's unexported-surface trim was attempted on 2026-08-08 and
reverted: the free functions below have zero internal and zero external
callers — they are fully dead public API, not `ConfigStore`-internal helpers.
Narrowing them to `pub(crate)` surfaces dead-code lints, and deleting them
would remove documented migration behavior. They remain public, unused, and
harmless.

A quality pass reads this table before it proposes removing public surface.
Each row is one keep decision; the gate fails if a listed item disappears from
its owner's source while the row stays. "Reviewed" is the date the decision was
last checked against the listed evidence.

| Item | Crate or package | Why retained | Evidence | Reviewed |
| --- | --- | --- | --- | --- |
| `discover_legacy_storage`, `plan_storage_transition`, `apply_storage_transition_cleanup`, `encode_backup_export_archive` | longhorn-config | dead public API today, but deleting them removes documented storage-transition and backup-export behavior | contract 004 § Profile Bootstrap And Transition; § Archive Format | 2026-11-22 |
| `new_toml`, `DocumentFormat::Toml` | longhorn-config | opt-in human-written TOML domains behind the `toml` feature; held, not selectable | contract 004 § Human TOML Domains | 2026-11-22 |
| `execute_grouped_adapter_restore` | longhorn-config | grouped adapter restore is a specced capability whose plan/execute/recover set has no product caller yet | contract 004 § Custom Backup And Restore Adapters | 2026-11-22 |
| `restore_safety_pin` | longhorn-config | only public source of the operation pin that retention preserves | contract 004 § Archive Publication And Retention | 2026-11-22 |
| `SurfacePresentation::is_regional` | longhorn-surfaces | companion to `focused_panel`; reads the recorded presentation the contract defines | contract 002 § Presentation | 2026-11-22 |
| `LayoutDefinitionRegistry::eligible_regions` | longhorn-surfaces | drag reveal needs eligible regions from Rust; TypeScript may not invent placement | contract 002 § Region Visibility | 2026-11-22 |
| `validate_normalized_document`, `validate_normalized_layout` | longhorn-surfaces | only producer of `NonCanonicalDocument` for canonical durable and default documents | contract 002 § Layout Normalization | 2026-11-22 |
| `load_or_default`, `LayoutFallback` | longhorn-surfaces-config | opt-in layout fallback the operator decision in contract 002 records | contract 002 § Layout Normalization | 2026-11-22 |
| `ConsumerSettingsTransactionAuthority`, `ConsumerSettingsTransactionOutcome` | longhorn-settings-config | consumer transaction authority that carries the failure-atomicity promise | contract 005 § Apply Units | 2026-11-22 |
| `PanelTransferOperation::Copy`, `CopyUnsupported` | longhorn-transfer | deferred copy and its specced rejection | contract 011 § Panel Commit | 2026-11-22 |
| `TransferAbort.retryable` | longhorn-transfer | retry class travels on host aborts | contract 010 § Names And Envelopes | 2026-11-22 |
| `AssembledTransferCommands::from_shared` | longhorn-tauri-transfer | optional Surface commands over the same transfer coordinator | contract 011 § Implementation Evidence | 2026-11-22 |
| `publish_registry_changed`, `publish_scope_changed` | longhorn-tauri-settings | only Rust emitters of the recomposition events the TS client listens to | contract 005 § Session And Navigation | 2026-11-22 |
| `plan_tauri_window_restore` | longhorn-tauri-windowing | named adapter entry point for window restore | contract 009 § Host Boundary; `../knowledge/architecture/tauri-window-host-integration.md` | 2026-11-22 |
| `ConfigWindowPlacementSink::rewrite_migrated_domain` | longhorn-windowing-config | only route through the placement sink for a migrated domain rewrite | contract 002 § Layout Persistence; contract 004 § Read And Migration | 2026-11-22 |
| `CorrelationAmbiguity.observation_ids` | longhorn-display | serialized wire shape matching `DisplayAvailability::Unresolved` | contract 009 § Display Identity | 2026-11-22 |
| `HistoryNavigationLimits::maximum_recent_plans`, `MAXIMUM_RECENT_HISTORY_PLANS` | longhorn-history | bounded per-session ring that rejects duplicate plan ids at planning time | `../knowledge/architecture/history-composition.md` | 2026-11-22 |
| `LinearHistory::with_navigation_limits`, `from_state_with_navigation_limits` | longhorn-history | constructor set for navigation limits, which also set the recent-plan ring | contract 008 § Limits And Pruning | 2026-11-22 |
| `NativeSurfaceProvider`, `NativeSurfaceAction` | longhorn-agent-control | provider seam for GPUI and native content; no provider ships under the contract | contract 022 § Boundaries | 2026-11-22 |
| `AgentControlShutdownError::AlreadyShutdown` | longhorn-tauri-agent-control | public variant of the shutdown error; no longer produced for a server thread that exited early, which now joins and reports its own result. Kept because removing a variant breaks the public enum | `../guides/agent-control-composition.md` § Hook Both `ExitRequested` And `Exit` | 2026-11-24 |
| `DeferralCause::InstallationNotWritable`, `ExternallyManaged`, `InstallFailed` | longhorn-update | public domain and wire variants retained for callers that construct and inspect deferrals; renderer commands cannot supply them | contract 018 § Staged Download And Install | 2026-11-27 |
| `ArtifactDigest` | longhorn-update | digest travels with the staged bytes and names the applied artifact | contract 018 § Staged Download And Install | 2026-11-22 |
| `DecliningSystemBrowser` | longhorn-browser | browser port paired with `BrowserLaunchError::Declined`; rejection policy stays with the app | contract 020 § Delegated Capabilities | 2026-11-22 |
| `native_platform_directory_fact*`, `resolve_storage_root`, `resolve_storage_bootstrap_paths` | longhorn-storage-layout | single-root resolver and one-fact provider; the bootstrap path join is the documented way in | contract 004 § Roots And Paths; `../guides/storage-configuration-backup.md` | 2026-11-22 |
| `is_remote_credential_operation`, `redact_git_detail` | longhorn-git-workspace | contract 029's public API list names both; Acowtancy uses both | contract 029 § Public API | 2026-11-22 |
| `TokenClient` | longhorn-git-workspace | transport and branding belong to the app, not the crate | contract 029 § Public API | 2026-11-22 |
| `Usability::warrants_attention` | longhorn-licence | separate policy question from `is_usable`; `longhorn-poodle` renders its banner from it | contract 019 § Enforcement | 2026-11-22 |
| `CredentialSlotError::NameTooLong` | longhorn-core | separate segment and total bounds, each with its own fixture | contract 021 § Contract | 2026-11-22 |
| `./<domain>/protocol` subpath exports, `./package.json` | `@inflatable-cookie/longhorn` | wire-contract-only surface: generated types and compatibility guards alone, plus the package manifest every resolution reads | contract 012 § Boundary; `../knowledge/architecture/package-topology.md` | 2026-11-22 |
| `./agent-control` export | `@inflatable-cookie/longhorn` | soundcheck composes the agent-control client entrypoint | contract 022 § Protocol; `../knowledge/architecture/repo-authority-map.md` | 2026-11-22 |
| `NotificationController` | `@inflatable-cookie/longhorn` | keeps the renderer's observation and command surface while owning its subscription and pending state | contract 016 § Snapshot And Events | 2026-11-24 |
| `DomainId::MAX_BYTES`, `DomainId::SEPARATOR`, `DomainId::SEGMENT_INITIAL`, `DomainId::SEGMENT_ADDITIONAL` | longhorn-core | Rust validation and TypeScript bindings share the declared grammar and byte bound | contract 010 § Boundary Validation Target | 2026-11-24 |
| `evidence/` | `examples/agent-control-proof` | required evidence for the agent-control contract | contract 022 § Required Evidence | 2026-11-22 |

Decision still deferred for the four `longhorn-config` migration and archive
functions above: contract them with a real caller, or delete when retention or
storage-transition work next touches them. `resolve_storage_bootstrap_paths`
moved to `longhorn-storage-layout` and is kept by the storage-layout row.
