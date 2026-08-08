# 059 Config-backed Keymaps And Generated Host Protocol

Status: complete
Owner: Tom
Roadmap: g01.010 batch 2
Governing refs: contracts 004, 006, 010, and 012; research memo 014
Depends on: Card 058
Auto-start next card: no
Completed: 2026-07-30

## Objective

Persist active preset selection and sparse keymap overrides through coordinated
configuration, generate the checked command/keymap protocol, and expose narrow
Tauri catalogue/keymap query and mutation handlers without generic command
execution.

## Scope

- `longhorn-command-config`
- registered versioned active-preset and override domain
- keymap load, preview, commit, reset, migration, recovery, and durability
- registry-generation and expected-keymap-revision checks
- patch digest binding conflict preview to commit
- `longhorn-bindings` command/keymap generation and golden fixtures
- `@inflatable-cookie/longhorn-commands` generated protocol and compatibility validation
- `longhorn-tauri-command` injected catalogue/keymap host assembly
- exact read and mutate command/event names plus capability examples

## Public Behavior

Keymap mutation rereads current configuration under coordination, validates the
current registry and preset, applies one typed patch, rejects conflicts, and
publishes one authoritative snapshot and exact durability receipt.

Preview and commit name the same base revision, registry generation, and patch
digest. Stale evidence returns fresh state. Session state changes only after
the published receipt.

The Tauri adapter exposes registry and keymap snapshots, conflict preview,
commit, and reset. It does not execute a product command.

## Out Of Scope

- product command execution
- renderer availability authority
- Svelte or Poodle UI
- settings shell registration
- server-synchronized keymaps
- donor repository migration

## Steps

1. Register the versioned keymap configuration domain.
2. Define load, preview, commit, reset, snapshot, receipt, and failure types.
3. Bind mutation to current registry generation and keymap revision.
4. Bind preview to commit through a canonical patch digest.
5. Reuse coordinated publication, migration, recovery, and durability.
6. Generate Rust-authoritative TypeScript and golden semantic fixtures.
7. Add checked compatibility and zero-diff regeneration.
8. Assemble injected Tauri catalogue/keymap query and mutation handlers.
9. Publish narrow permissions and read-only versus mutable capabilities.
10. Audit absence of generic execution, bridge, settings, Svelte, and Poodle
    dependencies.

## Acceptance Criteria

- active preset and sparse directives are one versioned user-config domain
- mutation rereads under the existing coordination authority
- changed registry, preset, keymap, or patch evidence fails stale
- previewed conflict state cannot drift into a different committed patch
- invalid, reserved, ambiguous, or unknown-command overrides never publish
- persistence failure leaves authoritative effective keymap unchanged
- migration and recovery preserve source evidence
- Rust and TypeScript round-trip every protocol discriminant
- generated files pass zero-diff regeneration
- Tauri capabilities separate read and mutate access
- no Tauri handler accepts a command id for execution

## Evidence Required

- coordinated mutation and failure-invariance matrix
- preview/commit race and patch-digest fixtures
- migration, future-schema, recovery, and durability fixtures
- Rust/TypeScript golden protocol and regeneration check
- Tauri mock-runtime handler and capability tests
- dependency, IPC-name, payload, and execution-bus audits
- focused Rust, frontend, and Effigy checks

## Stop Conditions

- mutation bypasses fresh config coordination
- preview cannot bind exact commit intent
- session-local success must precede persistence
- command execution enters the Tauri adapter
- generated TypeScript requires handwritten duplicate DTOs

## Next Task

Card 060 is ready. Build framework-neutral clients and per-instance UI state
over the checked catalogue and keymap boundary.

## Result

`longhorn-command-config` now registers active preset identity, monotonic
keymap revision, and sparse disable/replace/add directives as one ordinary
user-config domain. Loads retain default, file, migrated, recovery, and
unavailable posture. Migration preserves source versions and bytes. Backup
participation remains explicit through the shared catalogue.

Preview rereads coordinated state, checks registry generation, keymap
revision, active preset identity/version, canonicalizes patch order, and
returns a SHA-256 patch digest. Commit rereads under `mutate_checked`, requires
the exact preview evidence and digest, recompiles the candidate, and publishes
only valid conflict-free state. Reset uses the same evidence gate. Revisions
advance only for changed effective state.

The failure matrix proves one winner for concurrent commits, fresh stale
snapshots for digest or authority mismatch, corrupt/future recovery, in-memory
migration, atomic publication failure invariance, and compiled-default reset.
Invalid, unknown-command, ambiguous, and already-stale changes do not create
or alter the domain file.

`longhorn-bindings` now generates the command/keymap protocol and Rust-produced
golden fixture. `@inflatable-cookie/longhorn-commands` exposes the generated protocol and
fail-closed compatibility guards with no runtime dependency. Regeneration is
zero-diff and package imports touch no browser, Tauri, Svelte, Poodle, or
execution global.

`longhorn-tauri-command` exposes only catalogue, keymap load, preview, commit,
and reset. Read and mutation permissions are separate. Caller authorization
remains injected. Catalogue/keymap events are revision hints, not durable
delivery. No handler accepts a command id for product execution.

## Validation

- `effigy test:command-core`
- `effigy test:command-config`
- `effigy check:command-bindings`
- `effigy test:commands-ts`
- `effigy check:commands-package`
- `effigy test:tauri-command`
- `cargo clippy -p longhorn-core -p longhorn-command -p longhorn-command-config -p longhorn-bindings -p longhorn-tauri-command --all-targets -- -D warnings`
- `RUSTDOCFLAGS='-D warnings' cargo doc -p longhorn-command -p longhorn-command-config -p longhorn-tauri-command --no-deps`
- `bun x tsc -p packages/commands/tsconfig.json`
- 13 coordinated config fixtures, 4 checked TypeScript fixtures, and 7 Tauri
  handler/capability fixtures
