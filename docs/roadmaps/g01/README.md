# g01 Shared Desktop Foundation And Adoption

## Generation Runway

| Milestone | State | Outcome |
| --- | --- | --- | --- |
| [g01.001](001-foundation-contracts-and-package-topology.md) | complete | contracts and package graph |
| [g01.002](002-configuration-backup-and-recovery.md) | complete | versioned domains, safe writes, backup, restore |
| [g01.003](003-display-geometry-and-window-planning.md) | complete | pure display, coordinates, geometry, window plans |
| [g01.004](004-tauri-window-host-and-lifecycle.md) | complete | native window apply and lifecycle |
| [g01.005](005-layout-container-region-and-panel-core.md) | complete | Surface-independent layout state |
| [g01.006](006-optional-surfaces-and-cross-window-drag.md) | complete | optional full hosting and transfer |
| [g01.007](007-typescript-svelte-poodle-and-app-shell.md) | complete | checked clients and thin UI adapters |
| [g01.008](008-settings-registry-and-shell.md) | planning gate | centralized composable settings |
| [g01.009](009-typed-bridge-and-optional-backend-topology.md) | researchable | direct/local/remote semantic seam |
| [g01.010](010-command-registry-keymaps-and-palette.md) | blocked | commands, input, keymaps, palette |
| [g01.011](011-history-kernel-and-branching-prototype.md) | researchable | proven linear kernel, fork decision |
| [g01.012](012-async-operations-and-notifications.md) | incubation | jobs, progress, cancellation, notifications |
| [g01.013](013-native-content-islands-prototype.md) | prototype | child webview/native/render host seam |
| [g01.014](014-nucleus-no-surface-migration.md) | blocked | first simple consumer |
| [g01.015](015-loophole-full-hosting-migration.md) | blocked | advanced full-stack consumer |
| [g01.016](016-secondary-consumers-and-greenfield-release.md) | blocked | Soundcheck, Bovine, Jetstream, first release |

## Dependency Shape

```text
001 contracts/package graph
 ├─ 002 configuration ─┬─ 003 display/window plan ─ 004 Tauri window host
 │                     ├─ 005 layout core ─ 006 optional Surfaces/drag
 │                     └─ 008 settings
 ├─ 009 bridge/topology ─ 010 commands/input/palette
 ├─ 011 history research
 ├─ 012 async operations research
 └─ 013 native islands prototype

004-010 ─ 014 Nucleus ─ 015 Loophole ─ 016 secondary consumers/release
```

Research/prototype work in 009, 011, 012, and 013 may run beside foundation
implementation after their named contract questions are bounded. Promotion,
not research activity, gates dependent implementation.

## Active Milestone

`g01.005` through `g01.007` are complete. No execution card is ready.

`g01.002` is complete. The delivered storage foundation includes domain
storage, coordinated mutation, backup, restore, encrypted envelopes, custom
adapters, versioned platform layouts, fixed bootstrap selection, journaled
profile transition, legacy discovery, and receipt-bound cleanup.

`g01.003` is complete. Cards 017 and 018 provide checked Tauri observation,
explicit managed identity, injected dynamic creation, ordered native mutation,
partial receipts, and fresh convergence readback. Card 019 adds pure
apply/user event attribution, settling, debounce, bounded flush, close, and
destroy directives. Card 020 adds Tauri capture, injected persistence, reveal
gating, bounded close, and aggregate shutdown. Card 021 adds reusable mock and
native assembly, donor-shaped composition proofs, minimal capability policy,
fault receipts, and idempotent teardown. Card 022 adds the Rust 1.85-compatible
locked graph and packaged macOS arm64 restore, reveal, capture, restart,
dynamic-window, protected-primary, and flush evidence. `g01.004` is complete.

Read-only Loophole and Nucleus layout revalidation is promoted through research
memo 009 and contract 014. Cards 023-027 cover pure layout identity and policy,
authoritative mutation, registered persistence, generated TypeScript, and
two-shape conformance. Card 023 now supplies the pure registered model,
normalization, sizing, and visibility foundation. Card 024 supplies atomic
expected-revision mutation, authoritative receipts, failure invariance, and
opt-in bounded replay. Card 025 supplies registered configuration persistence,
registry-digest migration policy, fresh coordinated publication, bounded
presentation debounce, explicit flush, and independent layout/window domains.
Card 026 supplies checked Rust-to-TypeScript generation, compatibility guards,
golden fixtures, exact ratio and ordinary-visibility helpers, and a
framework-neutral package. Card 027 supplies checked Loophole and Nucleus
composition, one shared resolver/mutation matrix, exact TypeScript expected
snapshots, and package-boundary evidence.

Read-only Surface and transfer revalidation is promoted through research memo
010 and revised contracts 002 and 011. Cards 028-030 cover optional Surface
identity, lifecycle, persistence, and window-host composition. Card 031 adds
bounded sessions and leased targets. Cards 032-033 add same-document panel
move and whole-Surface transfer. Card 034 adds checked clients and Tauri host
assembly. Card 035 provides packaged proof and closeout. Cross-document panel
transactions, copy, and reusable UI adapters remain explicit later work.
Card 028 now supplies bounded Surface identity, strict topology, canonical
normalization, consumer-resolved presence, and deterministic available-window
fallback in a pure optional crate. Card 029 supplies authoritative lifecycle,
external container inventory evidence, exact active fallback, explicit cleanup
intent, registered persistence, migration and backup policy, fresh coordinated
publication, and three-domain isolation. Card 030 adds optional pure
Surface/window plan composition through the existing host, missing and
returning host behavior, readiness and shutdown evidence, Loophole hierarchy
conformance, and a no-Surface Nucleus dependency proof.
Card 031 adds the pure bounded transfer coordinator: exact injected session
identity and time, finite registries, atomic complete leases, epoch and destroy
invalidation, terminal single-use, fresh-bounds resolution, and deterministic
overlap rejection.
Card 032 adds fresh movable-panel admission, opaque direct and
Surface-container host bindings, same-domain and revision checks, and one
coordinated expected-revision `MovePanel` publication with exact failure
invariance.
Card 033 adds fresh whole-Surface admission, current target-policy checks,
expected-revision `MoveSurface` publication, exact external layout-binding
retention, and opt-in hidden provision with explicit cleanup and
reconciliation.
Card 034 adds checked Surface, transfer, and optional Surface-transfer
protocols, framework-neutral clients, listener-before-snapshot epoch
handling, narrow Tauri transport and handler assembly, managed-window
geometry projection, and minimal capability examples.
Card 035 adds separate direct and Surface-enabled Rust 1.85 packaged
artifacts. Real webviews exercise lease publication, panel and Surface
admission, explicit-zone and screen-point commits, empty-display window
provision, exact failure invariance, 2× scale boundaries, and package,
payload, capability, and authority audits. `g01.006` is complete.

Research memo 011 and revised contracts 011-013 promote the client, Svelte,
Poodle, drag, and shell boundary. Cards 036-041 cover a domain-neutral client
lifetime base, domain-free Tauri transport, per-window Svelte state, a named
Poodle public-seam and preview-artifact checkpoint, public layout bindings,
armed cross-window drag, shared titlebar behavior, and three distinct
artifact-installed shell shapes. Card 036 is complete with a shared checked
client lifetime, migrated Surface and transfer connections, and a domain-free
Tauri transport. Card 037 is complete with per-window rune state, consumer-fed
layout projection, optional domain subpaths, request-keyed reconciliation, and
exact mounted teardown. Card 038 is complete with Poodle's public drag seam
and exact preview artifact set. Card 039 is complete with Surface-free public
layout bindings, consumer-owned presentation snippets, serialized revisioned
mutation, and mounted Nucleus and Loophole shapes. Card 040 is complete with
armed panel and Surface transfer actions, protocol-only native payloads,
complete measured leases, projection-only reveal, authoritative target
commits, and injected titlebar behavior. Card 041 adds three isolated
artifact-installed shell graphs, public Poodle bootstrap, guarded reveal,
visible failures, narrow capabilities, and package-boundary audits.
`g01.007` is complete.

## Milestones

The complete known g01 suite is compiled above. Milestones are planning
envelopes, not execution authority. Cards 001 through 022 preserve the
completed configuration, display, and window-host runway. Cards 023-027 form
the completed layout runway. Cards 028-035 form the completed Surface and
transfer runway. Cards 036-041 form the completed client and UI-adapter
runway.

## Next Task

Resolve the post-g01.007 intent gate. Choose whether to compile the g01.008
settings runway or begin g01.009 bridge/topology research. Do not start
execution without a promoted ready card.
