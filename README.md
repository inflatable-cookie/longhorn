# Longhorn

Shared Rust and Svelte/TypeScript systems for Tauri desktop applications.

Longhorn extracts proven desktop mechanisms from existing apps, separates
product policy from reusable behavior, and supplies composable pieces for new
projects. It complements Poodle: Poodle owns visual primitives; Longhorn owns
desktop application state, host integration, and orchestration.

## Start Here

- [Docs](docs/README.md)
- [Vision](docs/vision/001-shared-tauri-systems.md)
- [Initial Tauri audit](docs/research/translation-memos/001-tauri-application-extraction-audit.md)
- [Shared system suite](docs/specs/001-shared-desktop-system-suite.md)
- [g01 roadmap suite](docs/roadmaps/g01/README.md)
- [Agent rules](AGENTS.md)

## Default Effigy Loop

```sh
effigy tasks
effigy doctor
effigy test --plan
effigy qa
```

## Current State

Strict-paused Northstar docs spine installed. The five-app audit, promoted
foundation contracts, package topology, and full g01 runway are preserved.
The configuration domain store and coordinated atomic patch mutation are
implemented. Bounded debounced mutation and explicit flush are complete.
Backup/archive implementation is paused on card 004's format, encryption,
snapshot, and atomic-restore contract.
