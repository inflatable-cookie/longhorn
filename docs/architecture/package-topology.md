# Package Topology

Status: promoted  
Owner: Tom  
Updated: 2026-07-29
Contract: `../contracts/012-distribution-and-compatibility.md`

## Repository Shape

```text
crates/       Rust domain libraries and host adapters
packages/     TypeScript, Svelte, and Poodle packages
tools/        binding and repository tools
examples/     composition and artifact-install proofs
fixtures/     cross-language and consumer-neutral fixtures
```

Cargo and npm workspaces share release metadata. Effigy owns repository task
discovery and validation entry points.

## Rust Packages

| Package | Responsibility | Depends on |
| --- | --- | --- |
| `longhorn-core` | ids, revisions, typed units, geometry, errors | none |
| `longhorn-config` | domains, roots, coordinated safe writes, backup, restore | core, `cap-std`, `fs4` |
| `longhorn-config-age` | optional authenticated binary age v1 backup envelopes | config, exact `age` adapter |
| `longhorn-display` | known/observed displays and correlation | core |
| `longhorn-windowing` | placement, desired/apply plans, pure event attribution and settling | core, display |
| `longhorn-windowing-config` | consumer-domain placement staging and coordinated flush | core, config, tauri-windowing |
| `longhorn-layout` | containers, regions, panels, normalization | core |
| `longhorn-layout-config` | registered layout domains, coordinated mutation, debounce, and flush | core, config, layout |
| `longhorn-surfaces` | optional Surface identity, topology, lifecycle, and pure host resolution | core |
| `longhorn-surfaces-config` | registered Surface domains and coordinated lifecycle publication | core, config, layout, surfaces |
| `longhorn-surface-windowing` | optional pure Surface/window plan projection and ordered shutdown | core, surfaces, windowing |
| `longhorn-transfer` | bounded sessions, leased targets, and deterministic target resolution | core |
| `longhorn-surface-transfer` | optional whole-Surface transfer and provision coordination | core, windowing, surfaces, transfer |
| `longhorn-command` | optional commands and input resolution | core |
| `longhorn-history` | optional history kernel | core |
| `longhorn-tauri-bridge` | typed IPC/event transport | core plus adapted domains |
| `longhorn-tauri-config` | mapping for Tauri-supplied platform paths and storage bootstrap | config; Tauri stays a consumer peer |
| `longhorn-tauri-windowing` | checked Tauri observation, managed identity, native mutation, lifecycle, capture, reveal, and flush | core, display, windowing, Tauri |
| `longhorn-tauri-transfer` | managed-window projection and transfer handler assembly | core, transfer, tauri-windowing, Tauri |
| `longhorn-bindings` | checked TypeScript generation | publishable domains |

The bindings tool is development-only. Additional adapter crates stay narrow;
there is no all-capabilities `longhorn-tauri` crate.

The first implemented generator slice is `longhorn-bindings layout`. It emits
the checked `@longhorn/layout` protocol and Rust-produced golden fixture.

## TypeScript Packages

| Package | Responsibility |
| --- | --- |
| `@longhorn/core` | generated shared protocol and framework-neutral utilities |
| `@longhorn/config` | generated config client and diagnostics |
| `@longhorn/windowing` | window snapshots and placement client |
| `@longhorn/layout` | generated layout snapshots, commands, receipts, and framework-neutral helpers |
| `@longhorn/surfaces` | optional Surface protocol |
| `@longhorn/transfer` | session, lease, target, and panel-transfer clients |
| `@longhorn/surface-transfer` | optional whole-Surface transfer client |
| `@longhorn/commands` | optional command/keymap clients |
| `@longhorn/history` | optional history client |
| `@longhorn/tauri` | raw transport implementation |
| `@longhorn/svelte` | reactive clients and actions |
| `@longhorn/poodle` | public Poodle bindings |
| `@longhorn/settings` | optional settings shell |

Names are working names until registry verification. Packages appear only
when their milestone implements a usable slice.

`@longhorn/layout` is the first implemented TypeScript package. It remains
private until registry authority is verified.

Its checked conformance artifacts cover Loophole and Nucleus shapes without
placing donor host identity or product payload in the published package.

## Dependency Rules

```text
core
├─ config
├─ display ─ windowing
├─ layout ─ layout-config ─ config
├─ surfaces ┬─ surfaces-config ─ config
│           └─ surface-windowing ─ windowing
├─ transfer ─ surface-transfer
│    ├─ windowing
│    └─ layout-config
├─ command
└─ history

domain packages -> narrow host adapters -> Svelte/Poodle presentation
```

- Arrows never point from a pure domain into a host or UI adapter.
- Optional packages cannot become dependencies of their foundations.
- Generated bindings live in the TypeScript package for their Rust domain.
- Poodle, Svelte, and Tauri remain peer dependencies at adapter edges.
- Product schemas and panel bodies never enter this graph.

## Examples

Examples arrive with the capability they prove:

- minimal configuration and shell
- `nucleus-no-surface-proof`: window-bound workspace without Surface packages
- full Surface hosting
- optional local or remote service
- installation from produced release artifacts

Local path dependencies are proof scaffolding only. Consumer migration targets
published prerelease artifacts with exact lockfile resolution.
