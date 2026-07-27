# System Inventory

Status: partial  
Owner: Tom  
Updated: 2026-07-27  
Architecture: `system-architecture.md`

## Coverage Summary

The target system is identified. Hosting and extraction boundaries are
contracted. Implementation planning remains blocked on detailed persistence,
window/display, IPC, and cross-window drag contracts.

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
| Local state store | Rust library | versioned local JSON lifecycle | Loophole, Nucleus, Soundcheck, Bovine | contract pending |
| Svelte/Poodle bindings | TS/Svelte package | state and interaction adapters | Loophole and Nucleus | contract pending |
| Tauri IPC/event bridge | Rust + TS package/tooling | checked command/event seam | all five apps | contract pending |
| Window chrome helper | TS utility | safe native titlebar drag | identical Loophole/Nucleus helpers | ready to include in Svelte contract |
| Command/keymap/palette | later package | action discovery and input resolution | Loophole full system; Jetstream basic shortcuts | research needed |
| Long-running job controller | later package | progress/cancel/listener lifecycle | Soundcheck | second consumer needed |
| Native content islands | later adapter family | native child/webview/embedded content geometry | Nucleus, Soundcheck, Jetstream | prototype needed |

## Consumer Repos

| Repo/app | Intended role |
| --- | --- |
| `loophole/aura` + `loophole/echo` | full hierarchy donor and advanced conformance consumer |
| `nucleus/apps/desktop` + `nucleus-workspaces` | no-Surface donor and simple conformance consumer |
| `soundcheck` | single-window persistence, job, event, and native-inspection specimen |
| `jetstream` editor | snapshot bridge, shortcut, and embedded-native-surface specimen |
| `acowtancy/bovine-accelerator-desktop` | greenfield-simple preference, split, tree-state, dialog specimen |
| `poodle` | external visual primitive authority |

## Planning Gaps

- canonical display identity portability outside Loophole
- logical versus physical coordinate contract
- local-state migration and corruption policy
- Rust-to-TypeScript contract generation/checking choice
- cross-window panel transfer semantics without Surfaces
- native-content-island common denominator
- release/package topology
