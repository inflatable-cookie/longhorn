# Contract Index

Status: active  
Owner: Tom  
Updated: 2026-09-15

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
  010, 013, and 022's semantic surface (its server core and capture sit in
  the host tier via a provider seam). A GPUI application composes none of
  it, and that is not a gap.

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
| [022 Agent App Control](022-agent-app-control.md) | compile-time opt-in MCP control surface: semantic input, capture, commands, and agent-answerable JS path selection | active; agent-answerable selection published in `0.2.1` |
| [023 Production Contextual Agent Tool Boundary](023-production-contextual-agent-tool-boundary.md) | transport-neutral typed dispatch/validation record; production MCP role withdrawn | withdrawn for production MCP; contract 022 opt-in server is the production MCP |

## Pending Contracts

Contract 023 is promoted only for provider-free L1 in g02.036, following
Desktop PR144 at `2a1bad3f` and Swallowtail Contract063. Production adoption
and packaged/live evidence remain held. Longhorn owns typed validation and
dispatch only; Desktop owns schemas and admission; Swallowtail owns registry
and operation lifecycle. L1 precedes the accepted-artifact D1 consumer.

Contract 022 was
promoted 2026-08-19 from Card 227's spike evidence
(memo 024): stateless mount and 2026-07-28 negotiation proved on the wire,
unfocused/occluded/minimized capture proved fresh, with the rAF/timer
caveat folded into the contract's `wait_for` semantics.

## Delivery Readiness

The contract register above is the contract-readiness authority. Delivery
sequencing, the open lanes, and the live next-task pointer belong to the
[generation index](../roadmaps/generation-index.md); this index does not carry
roadmap history.

- Contract 002 supersedes contract 014.
- Contract 022 is active as the compile-time opt-in agent-control surface; a
  consumer may ship it in a packaged build with `evaluate` omitted. Its
  agent-answerable `open`/folder/`save` amendment is implemented by g02.045;
  HTML file inputs and Rust-side pickers remain outside that route.
- Contract 023's production MCP role is withdrawn (operator direction
  2026-09-22). The contract 022 opt-in server is the production MCP.
- Contract 012's publication clause is live: `0.1.0` published 2026-09-16 —
  the three TypeScript packages on npm, `v0.1.0` tagged. Consumer repoint is
  outstanding.
