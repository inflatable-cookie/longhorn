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
| [g01.008](008-settings-registry-and-shell.md) | complete | centralized composable settings |
| [g01.009](009-typed-bridge-and-optional-backend-topology.md) | complete | direct/local/remote semantic seam |
| [g01.010](010-command-registry-keymaps-and-palette.md) | complete | commands, input, keymaps, palette |
| [g01.011](011-history-kernel-and-branching-prototype.md) | ready | typed linear kernel, lossless Loophole seam, private fork decision |
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
 ├─ 011 typed linear history + private fork decision
 ├─ 012 async operations research
 └─ 013 native islands prototype

004-010 ─ 014 Nucleus ─ 015 Loophole ─ 016 secondary consumers/release
```

Research/prototype work in 009, 011, 012, and 013 may run beside foundation
implementation after their named contract questions are bounded. Promotion,
not research activity, gates dependent implementation.

## Current Checkpoint

`g01.005` through `g01.010` are complete. Research memo 013 and compiled
contracts 007 and 010 promoted g01.009. Card 049 implements the pure
bridge negotiation and authority substrate. Card 050 implements typed
operations, retry/deduplication, ordered projections, and correlated jobs.
Card 051 adds checked TypeScript generation, strict clients, and
direct/serialized-loopback conformance. Card 052 adds checked registered-domain
Tauri host/client assembly, mock parity, and optional event support. Card 053
adds authority-gated lifecycle, bounded retry, session/epoch invalidation,
and optional injected supervision. Card 054 adds five-shape source
conformance, adapter parity, and exact optional-edge audits. Card 055 adds
five isolated artifact installs, separate Rust graph proofs, the composition
guide, and closeout. `g01.009` is complete. Research memo 014 and compiled
contract 006 define Cards 056-061. Card 056 implements the pure sealed
command/context registry, closed bounded arguments, deterministic discovery,
and shared search. Card 057 adds fresh availability, context/capability
revalidation, injected execution admission, typed outcomes, and bounded
evidence. Card 058 adds deterministic physical chords, immutable presets,
sparse directives, contextual resolution, gates, conflicts, and shortcut
projection. Card 059 adds coordinated keymap persistence, generated protocol,
and narrow Tauri hosting. Card 060 adds checked framework-neutral clients,
cross-language search and keyboard semantics, per-instance Svelte state,
public Poodle bindings, and capability-gated keybinding settings. Card 061
adds clean rich and minimal artifact installs, native and renderer semantic
traces, exact optional-edge and capability audits, composition guidance, and
closeout. `g01.010` is complete.

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

Research memo 012 and compiled contract 005 promote the settings boundary.
Cards 042-048 cover the pure registry and authority protocol, config-backed
one-domain apply units, generated TypeScript and Tauri host assembly,
per-instance Svelte state, a public-Poodle shell, shared storage/profile/backup
pages, exact restore/recovery flow, and artifact-installed composition proof.
Immediate and staged remain mutation timing; activation is a separate receipt.
Managed policy stays host authority. Cross-domain atomicity requires an
explicit consumer transaction authority. Card 042 now supplies the bounded
identities, sealed capability-gated registry, deterministic digest, authority
projection, commands, conflicts, and receipts. Card 043 adds the checked
configuration mutation seam and one-domain settings adapter with stale-token,
policy, reset, durability, recovery, and activation evidence. Card 044 adds
checked TypeScript generation, framework-neutral clients, registry helpers,
and narrow injected Tauri host assembly. Card 045 adds per-instance Svelte
state and one public-Poodle shell for modal, window, and panel hosts. Card 046
adds checked storage/backup operations, an injected Tauri edge,
capability-gated settings registration, and public-Poodle diagnostics and
operation pages. Card 047 adds checked restore inspection, explicit conflict
planning, exact terminal receipts, adapter separation, recovery gating, and a
public-Poodle destructive flow. Card 048 adds four isolated artifact-installed
compositions, exact optional-boundary audits, and the canonical composition
guide. `g01.008` is complete.

Research memo 013 promotes exact bridge negotiation, connection and host
identity, separate capability and domain authority, typed correlation,
ordered streams, job cancellation, retry/idempotency, indeterminate writes,
and injected service supervision. Cards 049-055 cover the pure Rust protocol,
generated TypeScript, direct and serialized-loopback conformance, narrow Tauri
assembly, reconnect/supervision lifecycle, five donor-shaped topology
fixtures, and artifact closeout. Production network transport, discovery,
authentication, service acquisition/update, and offline mutation queues stay
deferred.

Research memo 014 and compiled contract 006 promote the product-neutral
command boundary. Cards 056-061 cover a sealed command/context registry,
bounded arguments, fresh execution admission, deterministic physical-keyboard
resolution, config-backed sparse overrides, generated clients and narrow
keymap host assembly, per-instance Svelte state, public-Poodle projections,
and rich/minimal artifact proof. Command ids remain semantic input to injected
consumer executors; they do not become bridge or generic Tauri operations.
Macros, extended triggers, native accelerators, automation, and synchronized
keymaps stay deferred.

Research memo 015 and compiled contract 008 promote the history boundary.
Cards 062-067 cover typed payload policy, checked linear navigation,
coalescing and explicit groups, retention, structural persistence, committed
transition records, generated metadata clients, narrow Tauri assembly,
Svelte/Poodle composition, and rich/minimal artifact proof. Card 068 is a
non-publishable fork-tree prototype. Card 069 makes the explicit promote,
retain, or reject decision. Loophole keeps all DAW payload meaning, atomic
runtime apply, project snapshots and versions, autosave, journal file policy,
and recovery choice.

## Milestones

The complete known g01 suite is compiled above. Milestones are planning
envelopes, not execution authority. Cards 001 through 022 preserve the
completed configuration, display, and window-host runway. Cards 023-027 form
the completed layout runway. Cards 028-035 form the completed Surface and
transfer runway. Cards 036-041 form the completed client and UI-adapter
runway. Cards 042-048 form the completed settings runway. Cards 049-053 form
the implemented bridge protocol, direct/Tauri clients, lifecycle, retry, and
supervision substrate. Card 054 adds five-shape topology conformance. Card 055
adds artifact-installed proof and closes g01.009. Card 056 implements the
registry, context, argument, discovery, and search foundation. Card 057
implements fresh availability and injected execution admission. Card 058
implements deterministic keyboard and effective-keymap resolution. Card 059
implements coordinated persistence, checked protocol, and narrow Tauri
hosting. Card 060 adds projection and UI adapters. Card 061 adds isolated
artifact proof and closes g01.010. Card 062 adds typed policy, bounded
identity, validated linear state, record/coalesce outcomes, and two-shape
fixtures. Card 063 adds immutable navigation plans, injected atomic apply,
checked commit, failure invariance, and authoritative position receipts.
Card 064 adds explicit and injected-time grouping, count and encoded-weight
retention, exact baseline/future pruning, and authoritative metadata pages.
Cards 065-067 complete the public linear history runway. Card 068 is the
private fork prototype. Card 069 is the mandatory promotion decision and
g01.011 closeout.

## Next Task

Execute Card 065: structural persistence, explicit compatibility and recovery,
and committed transition records.
