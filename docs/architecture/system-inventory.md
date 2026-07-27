# System Inventory

Status: complete inventory; contracts partial
Owner: Tom  
Updated: 2026-07-27  
Architecture: `system-architecture.md`

## Coverage Summary

The known system suite, consumers, authority seams, and validation surfaces
are inventoried. Hosting, configuration, settings, commands, topology, and
history have first-pass boundaries. Implementation planning remains blocked
on package topology plus detailed display/window, IPC, and drag contracts.

The bootstrap `test` selector currently delegates to docs QA. Replace that
temporary test plan with native Rust and TypeScript discovery when the first
packages land.

## In-Scope Elements

| Element | Type | Authority | Evidence | Coverage |
| --- | --- | --- | --- | --- |
| Foundation model | Rust library | ids, geometry, normalization | Loophole Echo; Nucleus workspaces; Nucleus/Soundcheck window restore | architecture only |
| Display inventory | Rust library + Tauri adapter | canonical local display facts | Loophole `echo-configuration` and Aura local plane | contract pending |
| Window planning | Rust library | fallback and desired window plan | Loophole `echo-windowing`; `nucleus-workspaces` | contract pending |
| Tauri window host | Rust adapter | live apply, restore, event capture | Loophole Aura; Nucleus; Soundcheck | contract pending |
| Layout core | Rust + TS protocol | layout containers, regions, panels | Loophole Echo/Aura; Nucleus desktop/workspaces | first boundary contracted |
| Surface hosting | optional Rust + TS module | Surface lifecycle and window hosting | Loophole Aura/Echo | first boundary contracted |
| Local state store | Rust library | storage classes, domains, versioning, safe writes | Loophole, Nucleus, Soundcheck, Bovine | first boundary contracted |
| Backup and recovery | Rust library + adapters | inventory, verify, rotate, restore receipts | cross-app need; partial Loophole recovery | first boundary contracted |
| Svelte/Poodle bindings | TS/Svelte package | state and interaction adapters | Loophole and Nucleus | contract pending |
| Tauri IPC/event bridge | Rust + TS package/tooling | checked command/event seam | all five apps | contract pending |
| Window chrome helper | TS utility | safe native titlebar drag | identical Loophole/Nucleus helpers | ready to include in Svelte contract |
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
| Tauri path/window/monitor/event APIs | native desktop adapter | window and bridge contracts pending |
| local or remote service transport | optional product authority seam | contract 007; v1 transports pending |
| Poodle packages | component and presentation authority | public adapter contract pending |
| Rust/TS package registries | distribution and versioning | release topology pending |

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

- canonical display identity portability outside Loophole
- logical versus physical coordinate contract
- local-state migration and corruption policy
- backup archive, encryption, and multi-process locking
- Rust-to-TypeScript contract generation/checking choice
- cross-window panel transfer semantics without Surfaces
- settings transaction and restart-required semantics
- backend transport and offline mutation policy
- generic history payload and branching/checkpoint policy
- async operation/notification shared lifecycle
- native-content-island common denominator
- release/package topology
