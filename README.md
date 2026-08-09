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

Longhorn is pre-1.0 and private: packages are not on npm or crates.io. This
repo is also a working laboratory — most docs record how systems were built
and proven, not just how to use them. Start with the adoption guides below;
treat the rest as background.

## Start Here

1. [Vision](docs/vision/README.md) — why this exists
2. [Adoption guides](docs/guides/README.md) — choose packages, wire them in
3. [Glossary](docs/guides/glossary.md) — the terms used everywhere else
4. [Generated API surface](docs/reference/api-surface.md) — exact package names
5. [Docs index](docs/README.md)
6. [Current roadmap](docs/roadmaps/generation-index.md)
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

Pre-1.0 private workspace. Generation g01 is complete (20 milestones, cards
001-137, five migrated consumers: Nucleus, Loophole, Soundcheck, Split-shell,
Jetstream). Generation g02 is active, adding application-update and licensing
runways (cards 138-159). The private `0.1.0` compatibility candidate spans 18
TypeScript packages and 41 Rust crates; package-manager publication remains
deferred. The [generation index](docs/roadmaps/generation-index.md) owns the
live next-task pointer.
