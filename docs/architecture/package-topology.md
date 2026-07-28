# Package Topology

Status: promoted  
Owner: Tom  
Updated: 2026-07-28  
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
| `longhorn-display` | known/observed displays and correlation | core |
| `longhorn-windowing` | placement and pure desired/apply plans | core, display |
| `longhorn-layout` | containers, regions, panels, normalization | core |
| `longhorn-surfaces` | optional Surface hosting | core, windowing, layout |
| `longhorn-command` | optional commands and input resolution | core |
| `longhorn-history` | optional history kernel | core |
| `longhorn-tauri-bridge` | typed IPC/event transport | core plus adapted domains |
| `longhorn-tauri-windowing` | Tauri display/window host | display, windowing |
| `longhorn-bindings` | checked TypeScript generation | publishable domains |

The bindings tool is development-only. Additional adapter crates stay narrow;
there is no all-capabilities `longhorn-tauri` crate.

## TypeScript Packages

| Package | Responsibility |
| --- | --- |
| `@longhorn/core` | generated shared protocol and framework-neutral utilities |
| `@longhorn/config` | generated config client and diagnostics |
| `@longhorn/windowing` | window snapshots and placement client |
| `@longhorn/layout` | containers, regions, panels, transfer client |
| `@longhorn/surfaces` | optional Surface protocol |
| `@longhorn/commands` | optional command/keymap clients |
| `@longhorn/history` | optional history client |
| `@longhorn/tauri` | raw transport implementation |
| `@longhorn/svelte` | reactive clients and actions |
| `@longhorn/poodle` | public Poodle bindings |
| `@longhorn/settings` | optional settings shell |

Names are working names until registry verification. Packages appear only
when their milestone implements a usable slice.

## Dependency Rules

```text
core
├─ config
├─ display ─ windowing ─ surfaces
├─ layout ───────────────┘
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
- window-bound workspace without Surfaces
- full Surface hosting
- optional local or remote service
- installation from produced release artifacts

Local path dependencies are proof scaffolding only. Consumer migration targets
published prerelease artifacts with exact lockfile resolution.
