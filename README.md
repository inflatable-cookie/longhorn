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

Strict-ready Northstar docs spine installed. The five-app audit, promoted
foundation contracts, package topology, and full g01 runway are preserved.
The configuration domain store, coordinated atomic patch mutation, bounded
debounce, explicit flush, and coordinated backup capture are implemented.
Deterministic ZIP publication, bounded inspection, and safe retention are
implemented. Non-mutating restore inspection, exact conflict planning,
private staging, journaled publication, exact rollback, crash recovery,
coordinated load-sets, safe migration rewrite, and optional binary age v1
envelopes are implemented. Capability-declared custom adapters, truthful
external consistency groups, explicit adapter restore receipts, SQLite native
snapshot proof, and Loophole/Soundcheck/Bovine conformance fixtures are
implemented. Cross-platform storage identity, native/unified/portable layouts,
fixed bootstrap selection, journaled profile transition, receipt-bound source
cleanup, legacy discovery, root provenance, and Tauri-supplied path mapping
are implemented. `g01.002` is complete. `g01.003` display, geometry, and
window planning is at its card-compilation gate.
