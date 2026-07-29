# System Inventory

Status: complete inventory; g01.007 paused at Card 038 checkpoint
Owner: Tom  
Updated: 2026-07-29
Architecture: `system-architecture.md`

## Coverage Summary

The known system suite, consumers, authority seams, validation surfaces, and
package graph are inventoried. All foundation boundaries have first-pass
contracts. `g01.002` configuration, backup, recovery, and storage-layout work
is complete. `g01.003` has completed typed geometry, display correlation,
window placement resolution, and deterministic live diff planning. Cards
017-022 add checked Tauri observation, managed identity, native
execution, partial receipts, convergence readback, pure event
attribution/settling, settled capture, injected persistence, reveal gating,
bounded flush, reusable host composition, and packaged macOS proof. The layout
core now has promoted donor characterization, contract 014, an implemented
pure registry, durable state, normalization, sizing, and visibility
foundation, authoritative expected-revision mutation, registered persistence,
registry-digest migration policy, and bounded presentation debounce.
Checked Rust-to-TypeScript generation, compatibility guards, golden fixtures,
exact ratio and ordinary-visibility helpers, and package validation are also
implemented. Card 027 adds checked Loophole and Nucleus conformance through one
resolver and mutation engine, cross-language expected snapshots, and package
boundary evidence. `g01.005` is complete.
Research memo 010 and revised contracts 002 and 011 now bound optional Surface
identity, presence input, lifecycle, persistence, transfer sessions, leased
targets, same-document panel moves, whole-Surface moves, and explicit window
provision cleanup. Cards 028-035 complete g01.006. Card 028
implements bounded Surface identity, topology, normalization, presence input,
and available-window fallback. Card 029 implements authoritative lifecycle,
external container evidence, and registered coordinated persistence. Card 030
adds the optional pure Surface/window projection, existing-host mock
composition, missing and returning window evidence, readiness and partial
failure reconciliation, ordered shutdown, full-hierarchy conformance, and a
compile-only no-Surface dependency proof. Card 031 adds exact 128-bit injected
session identity, monotonic time, finite session/client/lease stores, atomic
complete replacement leases, epoch and destroy invalidation, typed expiry and
terminal replay, deterministic explicit-zone and screen-point resolution, and
overlap rejection. Card 032 now adds fresh panel admission, opaque
direct-window and Surface-container bindings, same-domain expected-revision
move publication, and exact abort invariance. Card 033 adds fresh
whole-Surface admission, ordinary and empty-display target resolution,
expected-revision Surface-only publication, exact layout-binding retention,
consumer target policy, and receipted provision, cleanup, and reconciliation.
Card 034 adds checked Surface and transfer protocol generation,
framework-neutral clients, epoch-safe renderer connection, narrow Tauri
transport and handler assembly, managed-window geometry projection, optional
Surface commands, and audited capabilities. Card 035 adds separate direct and
Surface-enabled packaged proofs, real multi-webview commands, explicit
empty-display provision, exact failure invariance, scale and boundary
evidence, and dependency, payload, capability, and authority audits.
`g01.006` is complete.
Research memo 011 and revised contracts 011-013 compile client lifetime,
domain-free Tauri transport, Svelte state, Poodle public bindings, armed drag,
titlebar behavior, and shell proof into Cards 036-041. Card 036 is complete
with the structural client lifetime and raw Tauri adapter. Card 037 adds the
Surface-free Svelte root, consumer-fed layout state, optional domain subpaths,
request-keyed optimism, and exact mounted teardown. Card 038 is the named
Poodle-local contract and artifact checkpoint.

## In-Scope Elements

| Element | Type | Authority | Evidence | Coverage |
| --- | --- | --- | --- | --- |
| Foundation model | Rust library | ids, geometry, normalization | Loophole Echo; Nucleus workspaces; Nucleus/Soundcheck window restore | typed geometry implemented |
| Display inventory | Rust library + Tauri adapter | canonical local display facts | Loophole `echo-configuration` and Aura local plane | pure inventory/correlation implemented |
| Window planning | Rust library | fallback and desired window plan | Loophole `echo-windowing`; `nucleus-workspaces`; Soundcheck | placement and live diff implemented |
| Tauri window host | Rust adapter | live apply, restore, event capture | Loophole Aura; Nucleus; Soundcheck | g01.004 complete through packaged Card 022 proof |
| Layout core | Rust + TS protocol | layout containers, regions, panels | Loophole Echo/Aura; Nucleus desktop/workspaces | g01.005 complete; donor-shaped foundation conformance passes without claiming migration |
| Surface hosting | optional Rust + config + TS packages | Surface lifecycle, persistence, and window hosting | Loophole Aura/Echo | pure identity, topology, presence, resolution, lifecycle, persistence, and window-host composition implemented |
| Cross-window transfer | Rust + TS protocol and narrow host adapters | bounded sessions, leased targets, authoritative move | Loophole Surface drag; shared panel need | g01.006 complete; direct and Surface-enabled packaged macOS proofs pass |
| Local state store | Rust library | storage classes, domains, versioning, safe writes | Loophole, Nucleus, Soundcheck, Bovine | first boundary contracted |
| Backup and recovery | Rust library + adapters | inventory, verify, rotate, restore receipts | cross-app need; partial Loophole recovery | first boundary contracted |
| Svelte/Poodle bindings | TS/Svelte package | state and interaction adapters | Loophole, Nucleus, Bovine, Poodle | Svelte state complete; paused at Card 038 Poodle checkpoint |
| Tauri IPC/event bridge | Rust + TS package/tooling | checked command/event seam | all five apps | contract 010 |
| Window chrome helper | TS utility | safe native titlebar drag | identical Loophole/Nucleus helpers | Card 040 |
| Settings registry/shell | Rust + TS/Svelte package | page composition and config transactions | Loophole plus cross-app demand | first boundary contracted |
| Command/keymap/palette | Rust + TS/Svelte packages | catalogue, context, input, palette projection | Loophole full system; Jetstream basic shortcuts | first boundary contracted |
| Optional backend topology | Rust traits + adapters | authority, capability, readiness, transport | Nucleus and Loophole process seams | first boundary contracted |
| History kernel | optional Rust package | generic linear navigation; branch prototype | Loophole Pulse | research boundary contracted |
| Long-running job controller | later package | progress/cancel/listener lifecycle | Soundcheck | second consumer needed |
| Notifications | later Rust + Svelte package | bounded records and presentation | Loophole, Soundcheck | research needed |
| Native content islands | later adapter family | native child/webview/embedded content geometry | Nucleus, Soundcheck, Jetstream | prototype needed |
| Greenfield starter | examples + docs | composition without donor baggage | Bovine and all future apps | roadmap |

## Consumer Repos

| Repo/app | Intended role |
| --- | --- |
| `loophole/aura` + `loophole/echo` | full hierarchy donor and advanced conformance consumer |
| `nucleus/apps/desktop` + `nucleus-workspaces` | no-Surface donor and simple conformance consumer |
| `soundcheck` | single-window persistence, job, event, and native-inspection specimen |
| `jetstream` editor | snapshot bridge, shortcut, and embedded-native-surface specimen |
| `acowtancy/bovine-accelerator-desktop` | greenfield-simple preference, split, tree-state, dialog specimen |
| `poodle` | external visual primitive authority |

## External And Host Surfaces

| Surface | Role | Planning state |
| --- | --- | --- |
| platform filesystem roots | config/data/cache/log/temp location authority | contract 004; adapter details pending |
| secure credential store | secrets outside ordinary config/backups | trait required; provider choice pending |
| Tauri path/window/monitor/event APIs | native desktop adapter | contracts 009-010 |
| local or remote service transport | optional product authority seam | contract 007; v1 transports pending |
| Poodle packages | component and presentation authority | contract 013; Card 038 public-seam and artifact checkpoint |
| Rust/TS package registries | distribution and versioning | contract 012; names pending verification |

## Validation Surfaces

- pure Rust unit and property fixtures
- serialization and schema-migration fixtures
- failure-injected filesystem and backup/restore tests
- direct/serialized backend conformance
- Vitest and Svelte component/adaptor coverage
- Tauri mock runtime command/event tests
- packaged desktop proofs for display/window/drag/native behavior
- consumer conformance fixtures and migration acceptance

## Planning Gaps

- settings transaction and restart-required semantics
- backend transport and offline mutation policy
- generic history payload and branching/checkpoint policy
- async operation/notification shared lifecycle
- native-content-island common denominator
- cross-document panel transaction and copy-transfer authority
- non-macOS strong display and packaged transfer evidence
