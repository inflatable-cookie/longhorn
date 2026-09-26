# Longhorn

Shared Rust and Svelte/TypeScript systems for Tauri desktop applications.

Longhorn extracts proven desktop mechanisms from existing apps, separates
product policy from reusable behavior, and supplies composable pieces for new
projects. It complements Poodle: Poodle owns visual primitives; Longhorn owns
desktop application state, host integration, and orchestration.

## What This Is For

You are the right audience if you build a Tauri desktop app in Rust with
Svelte/TypeScript and need any of: config storage with backup/restore,
window and display management, a settings system, commands and keymaps,
history, or native-content hosting.

Longhorn is pre-1.0. The three TypeScript packages publish to npm under
`@inflatable-cookie`; the Rust crates set `publish = false` and are taken by
git tag. Start with the adoption guides below; the knowledge base records the
architecture and contracts behind them.

## Start Here

1. [Vision](docs/knowledge/vision.md) — why this exists
2. [Adoption guides](docs/guides/README.md) — choose packages, wire them in
3. [Glossary](docs/guides/glossary.md) — the terms used everywhere else
4. [Generated API surface](docs/reference/api-surface.md) — exact package names
5. [Current state](docs/README.md)
6. [Plan](docs/plan.md)
7. [Agent rules](AGENTS.md)

## Effigy

This repo is run through **effigy**, a task runner. All commands below assume
it is installed; every selector is defined in `effigy.toml`, and the command
cookbook lives in `scripts/README.md`. There is no documented way to build or
test this repo without it.

## Default Effigy Loop

```sh
effigy tasks
effigy doctor
effigy test --plan
effigy qa
```

## Current State

Pre-1.0. Five consumers migrated: Nucleus, Loophole, Soundcheck, Split-shell,
Jetstream. The three TypeScript packages are on npm under `@inflatable-cookie`,
and the Rust crates enumerated by the
[generated API reference](docs/reference/api-surface.md) are taken by git tag.
[docs/README.md](docs/README.md) has the current release and what is being
built.
