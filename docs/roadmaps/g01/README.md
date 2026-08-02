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
| [g01.011](011-history-kernel-and-branching-prototype.md) | complete | public linear slice plus promoted private fork decision |
| [g01.012](012-async-operations-and-notifications.md) | complete | separate operation and notification authorities |
| [g01.013](013-native-content-islands-prototype.md) | complete | split production graph promoted from private evidence |
| [g01.014](014-nucleus-no-surface-migration.md) | complete | Surface-free Nucleus migration proved |
| [g01.015](015-loophole-full-hosting-migration.md) | complete | advanced full-stack consumer |
| [g01.016](016-secondary-consumers-and-greenfield-release.md) | executing; Card 126 ready | Soundcheck, Bovine, Jetstream, greenfield, private candidate |
| [g01.017](017-optional-forkable-history-tree.md) | planned | optional production fork-tree layer after linear adoption |
| [g01.018](018-native-content-production-and-adoption-gate.md) | complete | isolated artifacts, three-shape parity, packaged support, and adoption gates proved |

## Dependency Shape

```text
001 contracts/package graph
 ├─ 002 configuration ─┬─ 003 display/window plan ─ 004 Tauri window host
 │                     ├─ 005 layout core ─ 006 optional Surfaces/drag
 │                     └─ 008 settings
 ├─ 009 bridge/topology ─ 010 commands/input/palette
 ├─ 011 typed linear history + private fork decision
 ├─ 012 async operations + notification ledger
 └─ 013 native islands prototype ─ 018 native-content production

004-010 + 018 ─ 014 Nucleus ─ 015 Loophole ─ 016 secondary consumers/release
                                           └─ 017 optional fork tree
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
non-publishable fork-tree prototype. Card 069 promotes its proven semantics
into planned g01.017 while retaining the prototype as research. Loophole keeps
all DAW payload meaning, atomic runtime apply, project snapshots and versions,
autosave, journal file policy, and recovery choice.

Research memo 016 and compiled contracts 015-016 promote separate operation
and notification authorities. Cards 075-076 implement bounded operation
identity and progress, the exact lifecycle, receipted cancellation races,
count/weight retention, retry lineage, controlled teardown, and
Soundcheck/Loophole fixtures. Card 077 adds direct, serialized, Tauri, and
bridge-domain composition. Card 078 adds per-instance Svelte state and a
public-Poodle projection. Card 079 adds the independent retained ledger and
optional failure-isolated operation observer. Card 080 adds checked clients,
isolated Svelte state, retained panel/transient toast projections, and fresh
action admission. Card 081 adds four isolated artifact graphs, equal native
and renderer traces, multi-window/remount proof, and composition guidance.
`g01.012` is complete. Cards 070-074 remain planned
behind the linear-history adoption checkpoint.

Research memo 017 and contract 017 characterize native content islands as one
pure desired/observed coordination protocol over separate child-webview,
isolated native-window, and backing-surface mechanisms. The viewport is
semantic presentation and interaction geometry, not always a native child
frame. Cards 082-086 compile the private pure model, three independently
packaged mechanism proofs, and one promotion decision. Cards 082-085 prove the
pure model plus private child-webview, isolated-window, and backing-surface
mechanisms. Child-webview focus and visibility readback remain unknown. Native
scale switching remains unmet and unsimulated for child-webview and
backing-surface proofs on the one-monitor host. The isolated window passes its
full macOS matrix. Card 085 proves full-host storage, clipped output/input,
resize, stale invariance, destruction, and reversible detach. Card 086 selects
`Promote`, retains all prototypes as evidence, and compiles g01.018 Cards
087-093. Card 087 delivers the production pure kernel with exact generation,
planning, invalidation, proposal, and receipt semantics. Card 088 adds the
generated protocol, framework-neutral client, strict compatibility checks,
and direct/serialized/Tauri-shaped conformance. Card 089 adds the production
Tauri child-view package and packaged macOS proof. No donor migration is yet
claimed. Card 090 adds generic isolated-window coordination, bounded helper
correlation and teardown, and a passing 11-check packaged macOS proof. Card
091 adds generic backing-surface coordination, full-host storage versus clip
authority, deterministic output/input gates, and packaged macOS evidence with
the unavailable live scale transition recorded as unmet.

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
Card 065 adds the strict versioned envelope, independent structural/payload
migrations, explicit discard recovery, payload-free committed transitions,
and Loophole-shaped snapshot/journal recovery. Card 066 adds the generated
metadata protocol, checked clients, narrow Tauri assembly, per-instance
Svelte lifecycle, and public-Poodle panel. Card 067 adds isolated minimal and
Loophole-shaped artifact installs, equal native/renderer traces, exact
recovery and failure evidence, and composition/migration guidance. The public
linear runway is complete. Card 068 is the private fork prototype. Card 069
promotes its semantics, retains the private evidence, compiles g01.017, and
closes g01.011.

Cards 075-078 implement operation authority, checked transports, and
presentation. Card 079 implements the independent notification ledger and
operation observer. Card 080 implements its clients. Card 081 proves isolated operation-only,
Soundcheck-shaped, Loophole-shaped, and notification-only artifact graphs and
closes g01.012.

Cards 082-086 complete g01.013. Card 082 proves one private pure coordination
model and three product-neutral traces. Cards 083-085 prove child-webview,
isolated-window, and backing-surface mechanisms separately in packaged macOS
apps, with explicit target and dependency evidence. Card 086 promotes the
split graph and compiles g01.018 Cards 087-093. Card 087 now supplies the
production pure kernel. Card 088 now supplies the checked renderer protocol
and client. Card 089 now supplies the production child-view adapter and
packaged macOS evidence. Card 090 now supplies production isolated-window
coordination and matching packaged macOS evidence. Card 091 now supplies
production backing-surface coordination and matching packaged macOS evidence.
Card 092 now supplies isolated Svelte sessions, exact viewport measurement,
serialized explicit policy, remount invalidation, and public Poodle
composition without a package edge. Card 093 proves isolated Rust and
renderer graphs, equal traces, fresh packaged macOS evidence, and exact
consumer prerequisites. g01.018 is complete.

Cards 094-101 compile g01.014 into four batches: read-only donor freeze and
private artifact admission; canonical-id storage and protected-window cutover;
project-keyed no-Surface layout and renderer cutover; native Browser adoption
and conformance closeout. Card 094 is complete with
`pass_with_admission_gates`. Poodle geometry, the exact private
Longhorn/Poodle artifact graph, and both Nucleus layout checks are complete.
Card 096 moves desktop storage to canonical-id platform roots, imports the
retained `.nucleus` source through four explicit adapters, splits window and
layout stores, and commits the fixed locator last. Card 097 replaces the raw
geometry worker with a protected Longhorn host, canonical display-backed
placement, guarded reveal, and bounded flush. Fresh and restart native runs
converge. Card 098 transfers project-keyed layout structure to registered
Longhorn authority while retaining product presentation and runtime policy in
Nucleus. Card 099 completes the checked renderer and public Poodle cutover.
Card 100 completes the native Browser cutover with retained Nucleus policy and
packaged macOS attach/hide/reuse evidence. Card 101 closes exact artifacts,
restart, rollback, capability, duplicate-authority, retained-policy, and
Surface-absence conformance. g01.014 is complete.

Card 102 compiles g01.015 from a fresh read-only Loophole/Poodle audit. It
freezes the five-level hierarchy, regional and focused-panel habitats, generic
Echo disposition, stable `Loophole` storage identity, public drag seam,
settings/command split, 83-variant Pulse history boundary, and rollback order.
Cards 103-112 form the storage-policy, admission, foundation, hierarchy,
shell-system, linear-history, and closeout runway. Card 103 selects and proves
`shared-product-root-v1`: roaming AppData on Windows and exact `Loophole`
casing on every platform. Card 104 removes the stale direct Pulse SQLite
dependency, restores the clean donor baseline, and admits the exact private
Surface-enabled graph for Cards 105-111. Card 105's storage slice is complete.
Card 106 replaces display/window mechanics with a registered placement domain
and one protected/dynamic Longhorn host. Card 107 moves the literal regional
layout into registered Longhorn authority. Card 108 registers regional
Surface topology, lifecycle, host preference, restoration, and checked
whole-Surface movement while retaining focused-panel attachment policy in
Loophole. Card 109 completes the renderer, public Poodle, panel-transfer, and
whole-Surface transfer cutover. Card 110 completes settings, commands,
keymaps, palette, and retained extended input. Card 111 completes lossless
linear-history adoption. Card 112 closes exact source/artifact receipts,
restart/recovery, capabilities, duplicate authority, retained adapters, and
rollback posture. g01.015 is complete. Later systems and package publication
remain outside the admitted graph.

Cards 113-127 compile g01.016 into five batches. Cards 113-114 refresh exact
secondary-consumer authority and admit one private artifact graph. Cards
115-119 migrate and close Soundcheck across stable-name storage, protected
windowing, settings/recovery, scan operations, and isolated plugin windows.
Cards 120-121 complete Bovine's minimal config/settings graph, native restart,
rollback, settings lifetime, artifact, and no-optional-system proof while
preserving unrelated work. Card 122 completes Jetstream's checked bridge,
sealed commands, fresh admission, and physical keyboard. Card 123 completes
backing-surface coordination. Card 124 closes exact artifacts, peers,
capabilities, duplicates, retained engine/WGPU authority, and rollback. Card
125 proves four isolated produced-artifact compositions, exact optional-edge
removal, storage startup/mutation/reload, visible renderer lifecycle, and one
runtime per graph. Cards 126-127 add adoption guides and one deterministic private
`0.1.0` compatibility candidate. Registry publication, tags, and hosted
releases remain outside the chain.

## Next Task

Execute Card 126. Turn the proven compositions into the API, storage, backup,
composition, migration, and upgrade guide set.
