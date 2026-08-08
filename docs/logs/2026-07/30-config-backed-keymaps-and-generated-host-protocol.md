# Config-backed Keymaps And Generated Host Protocol

Date: 2026-07-30
Card: 059
Roadmap: g01.010

## Result

Added coordinated sparse keymap persistence, Rust-authoritative TypeScript
generation, strict compatibility checks, and a narrow injected Tauri host.
Product command execution remains outside every new boundary.

## Config Authority

One versioned user-config domain stores:

- monotonic keymap revision
- active immutable preset identity
- sparse disable, replace, and add directives

The registered domain binds one sealed registry, preset set, reserved-chord
policy, migration authority, and backup policy. Current state must compile to
a conflict-free effective keymap before load or publication succeeds.

Preview and commit reread through existing configuration coordination. Exact
evidence covers registry generation, keymap revision, preset identity/version,
and canonical patch digest. Patch order carries no digest authority. Changed
or contradictory evidence returns stale or rejected state without publication.

## Failure Invariance

The focused matrix proves:

- two commits from one preview produce one publication and one stale result
- changed patch digests do not publish
- changed registry, preset, or keymap evidence does not publish
- invalid, unknown-command, and ambiguous overrides do not publish
- corrupt and future sources retain recovery evidence
- explicit migration reports source and target versions without rewriting
- atomic publication failure preserves exact bytes and effective state
- reset returns to compiled defaults and advances revision once

Durability remains the shared config receipt. No session-local rebind precedes
publication.

## Generated Boundary

`longhorn-bindings` owns deterministic `commands write|check` generation.
Generated artifacts are:

- `packages/commands/src/generated/protocol.ts`
- `fixtures/commands/protocol-v1.json`

The fixture covers catalogue, preview/commit/reset requests, catalogue/keymap
events, all load/preview/mutation result statuses, all source origins, and
failure discriminants. `@inflatable-cookie/longhorn-commands` rejects future versions and unknown
discriminants. Its root is side-effect-free and framework-neutral.

## Tauri Boundary

`longhorn-tauri-command` exposes:

- `longhorn_command_catalogue`
- `longhorn_command_keymap`
- `longhorn_command_keymap_preview`
- `longhorn_command_keymap_commit`
- `longhorn_command_keymap_reset`

Read capability grants the first two. Mutation capability grants the last
three. Injected authority still checks the calling window. The adapter emits
`longhorn://command/catalogue-changed` and
`longhorn://command/keymap-changed` only as invalidation hints.

Source and dependency audits reject a generic execute endpoint, command-id
execution payload, bridge, settings, layout, Surface, transfer, Svelte, and
Poodle edges.

## Validation

- focused command core, config, binding, TypeScript, package, and Tauri Effigy
  selectors
- Clippy with warnings denied across all changed Rust packages
- rustdoc warnings denied for public command packages
- strict TypeScript check
- zero-diff command binding regeneration

## Next

Card 060 is ready. Build injected framework-neutral clients, per-instance
Svelte sessions, shared projections, and public Poodle bindings over this
checked host state.
