# Vision: Shared Desktop Foundations

Status: active  
Owner: Tom  
Updated: 2026-11-28

## Purpose

Desktop apps repeatedly rebuild display discovery, window restore, local
preferences, settings, commands, panel layouts, and native-content
coordination. Longhorn is a [Rust desktop application framework with
pluggable Tauri and GPUI hosts](contracts/contract-index.md#positioning).
TypeScript and Svelte provide checked clients and presentation adapters for
webview compositions. Shared mechanisms let new apps start from tested
foundations while each app owns its product policy.

## Product Outcome

An app composes only the layers it needs:

- a simple app may use local preferences and one restored window
- a workspace app may add regions, panels, tabs, and split persistence
- a multi-window app may add display inventory and window fallback
- an advanced workstation may add hosted surfaces and cross-window movement
- any shape may add a registered settings shell, command palette, backup,
  history, or optional local/remote backend without adopting the rest

Apps that use registered regions and panels follow the same
[hosting chain](contracts/002-composable-workspace-hosting.md#hosting-composition).
A Surface owns the layout. An app can present named Surfaces to users or keep
one unlabelled layout Surface per window. An app that needs only preferences
and a window can omit layout entirely.

## Strategic Constraints

- Rust owns durable domain rules and host integration.
- TypeScript owns framework-neutral client projection where needed.
- Svelte integrations bind Longhorn state to Poodle components.
- Poodle remains the visual-component authority.
- App-specific panel kinds, workflows, resources, and business state stay in
  consumer repos.
- Platform-specific behavior sits behind explicit adapters.
- Optional capability packages beat one mandatory all-purpose shell.
- Shared APIs need evidence from multiple apps or a strong mechanism-level
  greenfield case.
- Consumer migration and removal of donor copies are part of extraction done.
- Configuration uses correct platform locations, schema-safe writes, and
  recoverable backups instead of per-app path conventions.
- Local desktop authority remains usable when an optional backend is absent.

## Success

- at least two materially different apps consume the foundation packages
- packaged compositions prove window-bound layout and full hosted-Surface layout
- window/display loss and restore are deterministic and tested
- Rust and TypeScript contracts do not drift silently
- new Tauri and GPUI apps can adopt a documented minimum stack
- settings, commands, and optional systems compose from registries rather than
  copied app shells
- configuration backup and restore can be inspected, verified, and scoped
- app-specific policy never leaks into the shared core

## Non-Goals

- a second component library
- a universal shell that every app must adopt
- moving domain-specific DAW, agent, accounting, or engine behavior into
  Longhorn
- preserving pre-1.0 donor APIs through aliases
