# 001 Shared Tauri Systems

Status: active  
Owner: Tom  
Updated: 2026-07-27

## Purpose

Tauri apps in `~/Dev/projects` repeatedly rebuild the same desktop shell
mechanisms: display discovery, window restore, local preferences, docked panel
layouts, drag/drop, settings, command/event bridges, history, service
connections, and native-content coordination.

Longhorn turns the reusable parts into maintained Rust and
Svelte/TypeScript libraries. Existing apps become real consumers. Greenfield
apps start from coherent mechanisms instead of copying another product.

## Product Outcome

An app composes only the layers it needs:

- a simple app may use local preferences and one restored window
- a workspace app may add regions, panels, tabs, and split persistence
- a multi-window app may add display inventory and window fallback
- an advanced workstation may add hosted surfaces and cross-window movement
- any shape may add a registered settings shell, command palette, backup,
  history, or optional local/remote backend without adopting the rest

Loophole can keep:

`display -> window -> surface -> region -> panel`

Nucleus can use:

`display -> window -> region -> panel`

Neither shape becomes a compatibility mode or a fork.

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
- Nucleus proves the no-Surface composition
- Loophole proves the full hosted-Surface composition
- window/display loss and restore are deterministic and tested
- Rust and TypeScript contracts do not drift silently
- new Tauri/Poodle apps can adopt a documented minimum stack
- settings, commands, and optional systems compose from registries rather than
  copied app shells
- configuration backup and restore can be inspected, verified, and scoped
- app-specific policy never leaks into the shared core

## Non-Goals

- a second component library
- a universal application framework
- forcing every app into Loophole's workspace hierarchy
- moving domain-specific DAW, agent, accounting, or engine behavior into
  Longhorn
- preserving pre-1.0 donor APIs through aliases
