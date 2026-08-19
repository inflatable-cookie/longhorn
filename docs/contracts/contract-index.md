# Contract Index

Status: active  
Owner: Tom  
Updated: 2026-08-16

## Positioning

Longhorn is a **Rust desktop application framework with pluggable host
backends**. Tauri and GPUI are both first-class and permanent; neither is the
reference implementation. Applications on either host compose the same
host-agnostic core.

## Contract Tiers

Contracts sit in one of three tiers, and the tier decides who must satisfy
them.

- **Core** — host-agnostic. Every application satisfies these whatever its
  backend: 001, 002, 003, 004, 005, 006, 007, 008, 011, 012, 014, 015, 016,
  018 (policy), 019, 021.
- **Host** — what a backend must provide: 009, 017, 020, and contract 018's
  execution half. Satisfied per backend, proved per backend.
- **Webview edge** — optional, and only meaningful where a webview exists:
  010, 013. A GPUI application composes none of it, and that is not a gap.

A claim proved on one backend does not close a host-tier contract.

## Contract Register

| Contract | Boundary | Status |
| --- | --- | --- |
| [001 Working Rules](001-working-rules.md) | delivery and refactoring | active |
| [002 Composable Workspace Hosting](002-composable-workspace-hosting.md) | optional Surface and shared layout core | active compiled boundary |
| [003 Extraction And Consumer Migration](003-extraction-and-consumer-migration.md) | donor admission, cutover, ownership | active |
| [004 Configuration Storage, Backup, And Recovery](004-configuration-storage-backup-and-recovery.md) | roots, domains, safe writes, migration, backup, ordinary and grouped custom restore | active compiled boundary |
| [005 Settings And System Registration](005-settings-and-system-registration.md) | registry, apply units, policy, activation, and shell composition | active compiled boundary |
| [006 Command, Action, And Input](006-command-action-and-input.md) | sealed registry, fresh admission, keyboard, durable keymaps, projections | active compiled boundary |
| [007 Optional Backend Topology](007-optional-backend-topology.md) | local/remote adapters, lifecycle, retry, and domain authority | active compiled boundary |
| [008 History Kernel Boundary](008-history-kernel-boundary.md) | typed linear history, atomic navigation, persistence seams, and private fork gate | active compiled boundary |
| [009 Display Identity, Coordinates, And Window Planning](009-display-identity-coordinates-and-window-planning.md) | display correlation, typed geometry, pure window plans | active first pass |
| [010 Rust, TypeScript, IPC, And Events](010-rust-typescript-ipc-and-events.md) | type authority, handler/client seam, correlation, revisions, lifecycle | active compiled boundary |
| [011 Cross-window Transfer](011-cross-window-transfer.md) | id-only sessions, leased targets, authoritative move | active compiled boundary |
| [012 Distribution And Compatibility](012-distribution-and-compatibility.md) | package graph, versions, artifacts, adoption | active compiled boundary |
| [013 Svelte And Poodle Adapter Lifecycle](013-svelte-and-poodle-adapter-lifecycle.md) | reactive lifetime and public component binding | active compiled boundary |
| [014 Layout Container, Region, And Panel Core](014-layout-container-region-and-panel-core.md) | superseded by 002; layout is Surface state since Card 179 | superseded |
| [015 Async Operation Lifecycle](015-async-operation-lifecycle.md) | finite lifecycle, progress, cancellation receipts, retention, and projection | active compiled boundary |
| [016 Notification Ledger And Projection](016-notification-ledger-and-projection.md) | independent retained records, seen/dismiss state, actions, and transient projection | active compiled boundary |
| [017 Native Content Island Coordination](017-native-content-island-coordination.md) | shared desired/observed coordination across separate native host mechanisms | active promoted production boundary |
| [018 Application Update And Release Channels](018-application-update-and-release-channels.md) | update policy, source adapters, channels, client-side rollout, restart readiness, cross-channel store compatibility | active compiled boundary |
| [019 Licensing, Entitlement, And Activation](019-licensing-entitlement-and-activation.md) | licence shape, trust basis, opaque entitlements, use/update windows, activation adapters, lease and fail-open | active compiled boundary |
| [020 Host Adapter Boundary](020-host-adapter-boundary.md) | what a backend must provide, what it may not do, delegated capabilities, dual-backend evidence | active compiled boundary |
| [021 Consumer-scoped Credential Slots](021-consumer-scoped-credential-slots.md) | validated built-in and consumer namespace/scope/purpose identities over one credential store | active compiled boundary |

## Pending Contracts

[022 Agent App Control](022-agent-app-control.md) is drafted, not compiled: a
dev-only, stateless MCP control server inside each app so agents drive a
running app (semantic snapshot, in-page input, unfocused screenshots, command
invocation) without OS focus or pointer theft. Needs a research memo and a
runway slot before promotion.

## Roadmap Readiness

`g01.001`, the configuration domain store, coordinated atomic mutation,
debounced mutation, explicit flush, and bounded coordinated backup capture are
complete. Verified ZIP publication and safe operational retention are
complete. Restore inspection, confirmation-bound planning, private staging,
journaled publication, exact rollback, crash recovery, coordinated load-sets,
safe migration rewrite, and optional binary age v1 envelopes are complete.
Custom backup adapters, separate consistency groups, SQLite native snapshot
proof, donor-shaped conformance, cross-platform storage profiles, fixed
bootstrap selection, journaled profile transition, legacy discovery, and
receipt-bound cleanup are complete. `g01.002` and `g01.003` are complete,
including typed geometry, display inventory/correlation, placement resolution,
and deterministic desired/live planning. The Tauri host boundary is complete
through Card 022 packaged macOS proof. Contract 014's layout-core foundation
is implemented through Cards 023-027 and `g01.005` is complete. `g01.006` is
complete through Cards 028-035. Contract 011
remains current after the Surface foundation checkpoint; bounded sessions,
leases, target resolution, same-document panel moves, whole-Surface moves,
opt-in provision cleanup, checked clients, and Tauri host assembly are
implemented. Card 035 adds passing direct and Surface-enabled packaged macOS
proofs and boundary audits. Research memo 011 and revised contracts 011-013
compile g01.007 into Cards 036-041. Cards 036-040 are complete. Poodle's public
drag seam and exact preview artifact are recorded. Card 039 completes the
Surface-free public layout binding slice. Card 040 completes armed transfer,
checked leases, compatible reveal, and titlebar actions. Card 041 completes
three isolated artifact-installed shell shapes and closes `g01.007`.
Research memo 012 and compiled contract 005 make Cards 042-048 the completed
g01.008 settings runway. Card 048 proves four artifact-installed compositions,
exact optional boundaries, and one Svelte/Poodle runtime.
Research memo 013 compiles contracts 007 and 010. Cards 049-055 implement,
artifact-prove, and close g01.009. Research memo 014 compiles contract 006
against the bridge, config, settings, Svelte, and Poodle foundations. Cards
056-061 and g01.010 are complete. Research memo 015 revalidates Loophole
history and compiles contract 008. Cards 062-069 and g01.011 are complete.
Research memo 016 promotes separate operation and notification authorities
through contracts 015-016. Cards 075-081 implement and artifact-prove both
systems; g01.012 is complete. Research memo 017 and contract 017 promote the
native-content coordination boundary. Cards 082-085 provide retained private
evidence. Card 086 selects the split production graph and compiles g01.018.
Cards 087-093 are complete. Nucleus Cards 094-101 complete its no-Surface
migration, including the public Poodle geometry seam, accepted project checks,
private artifacts, canonical storage, native host, renderer, Browser,
rollback, and closeout.

Loophole Card 102 freezes the current full hierarchy, generic Echo transfer,
stable product storage identity, public Poodle drag seam, and 83-variant Pulse
history boundary. g01.015 is compiled through Card 112. Cards 103-108 complete
the storage, artifact-admission, display/window, layout, and regional Surface
authority slices. Card 109 owns renderer and transfer composition.
g01 is complete through Card 137, including the secondary-consumer
migrations, the private compatibility candidate, the optional fork-tree
layer, grouped-adapter absence restore, and child-view navigation.
Generation g02 delivered the memo-018 remediation (Cards 138-147), the
update and licensing runways (g02.009, g02.010), and the memo-023
release-audit suite (cards 200-223) is the live runway. Publication is
scoped and queued: the `@inflatable-cookie` scope was claimed 2026-08-09 and
the v0.1.0 tag waits on Poodle v0.2.0 (g02.014).

Research memo 020 compiles contract 019 over licensing. Longhorn owns the
licence shape and its evaluation; applications own the backend, the purchase
model, and what an absent entitlement means. Two independently optional
windows — use and update — express subscription, perpetual-with-maintenance,
trial, and freemium without Longhorn naming any of them, and the update
window is what contract 018's updater consults before offering a release.
Longhorn answers "entitled?" and never enforces. Cards 155-158 (g02.010)
execute it.
