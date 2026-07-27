# Configuration Domain Store

Date: 2026-07-28  
State: complete implementation batch

## Outcome

- created the Cargo workspace with edition 2024 and MSRV 1.85
- implemented `longhorn-core` domain ids and schema versions
- implemented `longhorn-config` storage classes, injected roots, portable
  relative paths, typed locations, and duplicate-safe registration
- implemented missing/default, current-file, recovery, and sequential
  in-memory migration outcomes
- preserved source bytes for every file-backed ready or recovery result
- confined ordinary reads beneath capability-scoped roots
- added native Rust Effigy format, lint, and test selectors

## Public Behavior

- JSON envelopes use `domain`, `schemaVersion`, and `value`
- domain codecs own raw-version validation, current decoding, typed
  validation, and one-step migration
- future, corrupt, mismatched, invalid, incomplete-migration, and escaped-root
  reads never repair or replace source
- defaults and secrets cannot acquire ordinary file paths
- missing workspace, project, policy, or secure-store authority remains typed
  and unavailable

## Dependencies

- `serde` and `serde_json` for the Rust-authoritative envelope and codecs
- `cap-std` for cross-platform capability-scoped filesystem reads
- `tempfile` for test roots only

No Tauri, Svelte, Poodle, Surface, or consumer dependency entered the graph.

## Evidence

- 21 passing unit and acceptance tests
- source-preservation fixtures for migration and every invalid input class
- duplicate id and resolved-path fixtures
- portable path traversal rejection
- Unix symlink-escape rejection through the capability root
- `cargo +1.85.0 check --workspace --all-targets`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy test --plan`
- `effigy qa`

## Deferred Gate

Safe mutation needs a promoted multi-process lock model. Archive, compression,
encrypted backup, restore, TypeScript bindings, and Tauri root adapters remain
outside this batch.

## Next

Research and promote the multi-process lock model. Compile the safe-mutation
card only after that decision closes.
