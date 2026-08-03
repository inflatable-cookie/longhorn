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
- [Adoption guides](docs/guides/README.md)
- [Generated API surface](docs/reference/api-surface.md)
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

Generation g01 is complete: 20 milestones and Cards 001-137 delivered the
shared configuration, backup/restore, display/windowing, layout, optional
Surface and cross-window transfer, Svelte/Poodle client, settings, bridge,
command/keymap/palette, linear and optional fork-tree history, operation,
notification, and native-content systems, with packaged macOS proofs and
five migrated consumers (Nucleus, Loophole, Soundcheck, Bovine, Jetstream).
The private `0.1.0` compatibility candidate spans 18 TypeScript packages and
38 Rust crates; package-manager publication remains deferred.

Generation g02 (workspace integrity remediation) is active. Research memo
018 records the post-g01 audit; Cards 138-147 close layout serde
invariants, window-lifecycle event-loop and retag correctness, transfer
session truthfulness, off-main-thread storage commands, restore self-heal,
a best-effort diagnostics seam, and QA/docs alignment. The
[generation index](docs/roadmaps/generation-index.md) owns the live
next-task pointer.
