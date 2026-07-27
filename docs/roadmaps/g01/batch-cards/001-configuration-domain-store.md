# 001 Configuration Domain Store

Status: complete  
Owner: Tom  
Completed: 2026-07-28  
Roadmap: g01.002 batch 1  
Governing refs: contracts 001, 004, and 012; architecture package topology  
Auto-start next card: no

## Objective

Create the Rust workspace foundation and ship a typed, read-only configuration
domain store with injected roots, validation, diagnostics, future-schema
protection, and ordered in-memory migration.

## Scope

- scaffold the Cargo workspace, `longhorn-core`, and `longhorn-config`
- establish edition 2024, MSRV 1.85, shared lint posture, and Effigy Rust tasks
- define validated domain ids, schema versions, storage classes, roots, and
  confined relative paths
- register typed domains without duplicate ids or paths
- read missing, current, corrupt, mismatched, older, and future documents
- validate defaults and loaded values
- run sequential migration in memory without rewriting source files
- add unit, fixture, and temporary-root tests

## Serialized Document

JSON domain files use this envelope:

```json
{
  "domain": "example.preferences",
  "schemaVersion": 2,
  "value": {}
}
```

- Domain ids are dot-separated lowercase ASCII segments. A segment begins with
  a letter and then uses letters, digits, `_`, or `-`.
- Schema versions are positive integers.
- Relative file paths are explicit, normalized, below the resolved class root,
  and cannot contain an absolute prefix, `.` segment, or `..` segment.
- Unknown fields follow the domain codec. The store performs no generic merge.

## Public Behavior

- A typed domain supplies its descriptor, default, decoder, validator, and
  one-step migration functions.
- Registration rejects duplicate domain ids and resolved relative paths.
- Missing input returns a validated default plus a `missing` diagnostic.
- Current valid input returns a ready typed value.
- Older input migrates one version at a time, validates every output, and
  returns a migrated-in-memory outcome with the original bytes unchanged.
- Corrupt JSON, wrong domain id, invalid value, missing migration step, and
  future schema return typed recovery outcomes with the source bytes and path.
- Defaults-only, secure-store-required, and explicit project-root classes do
  not silently become ordinary files.
- No read outcome writes, renames, deletes, or repairs a source.

Internal Rust type and module names may vary. The serialized envelope and
observable outcomes above may not.

## Out Of Scope

- file writes, patches, transactions, debounce, and flush
- migration rewrite or pre-migration backup
- multi-process locking
- backup archives, retention, restore, and receipts
- secure-store provider implementation
- policy precedence and remote synchronization
- TypeScript bindings and Tauri root adapters

## Steps

1. Add workspace manifests and the two usable crates. Do not scaffold later
   capability packages.
2. Implement core ids, schema version, diagnostics support, and the config
   domain/roots/path model.
3. Implement registry checks and read outcomes.
4. Implement ordered in-memory migration and validation.
5. Add fixture and temporary-root tests for every public outcome.
6. Make Effigy discover format, lint, and Rust tests; run the complete batch
   validation.
7. Record closeout evidence and compile the safe-mutation card only if its
   locking decision is closed.

## Acceptance Criteria

- config, machine-state, workspace-local, cache, defaults-only, secret, and
  explicit-project fixtures resolve to distinct typed location outcomes
- path traversal and duplicate id/path registration fail before I/O
- missing input returns only a validated default and diagnostic
- current input decodes and validates
- older input migrates sequentially in memory and preserves source bytes
- corrupt, mismatched, invalid, incomplete-migration, and future input return
  typed recovery without filesystem mutation
- package graph contains no Tauri, Svelte, Poodle, Surface, or consumer
  dependency
- public behavior has rustdoc and tests

## Evidence Required

- unit and fixture tests covering every acceptance branch
- temporary-root proof that reads never escape the injected root
- `effigy test --plan`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`
- batch log with dependency and unresolved-decision notes

## Stop Conditions

- implementation requires changing storage-class authority or the serialized
  envelope
- a dependency requires raising the declared MSRV
- root confinement cannot be proven without choosing a host-specific API
- migration requires a destructive rewrite or backup policy in this card
- Effigy cannot expose the native Rust validation lane

## Continuation

Stop after closeout. The next card must close and name the multi-process lock
model before implementing mutation. Backup/archive work remains a later card.

## Completion Notes

- added Cargo workspace edition 2024 with declared MSRV 1.85
- added usable `longhorn-core` and `longhorn-config` crates
- added validated ids, schema versions, portable domain paths, roots,
  locations, registration, and typed load outcomes
- added capability-confined reads through `cap-std`
- added current, missing, recovery, and sequential in-memory migration paths
- added 21 unit and acceptance tests, including source-preservation and
  symlink-escape fixtures
- made Effigy discover Rust format, lint, and test selectors

## Next Task

Research and promote the multi-process lock model. Compile the safe-mutation
card only after that decision closes.
