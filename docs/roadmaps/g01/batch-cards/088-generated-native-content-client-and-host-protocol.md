# 088 Generated Native-content Client And Host Protocol

Status: complete
Owner: Tom
Roadmap: g01.018 batch 1
Governing refs: contracts 010, 012, 013, and 017; Card 086
Depends on: Card 087
Auto-start next card: no

## Objective

Generate a checked renderer protocol from production Rust authority and expose
a framework-neutral client over injected direct or Tauri transport.

## Scope

- Rust request, snapshot, observation, proposal, and receipt protocol
- checked TypeScript generation and golden fixtures
- `@inflatable-cookie/longhorn-native-content` framework-neutral package
- listener-first snapshot reconciliation and session epochs
- bounded operation correlation and stale-result rejection
- injected direct transport and narrow Tauri transport assembly
- capability examples without content authorization

## Out Of Scope

- mechanism construction or native operations
- Svelte or Poodle
- browser/plugin/GPU payloads
- generic command bus or arbitrary JSON escape hatch
- donor writes

## Acceptance Criteria

- Rust owns every wire shape and compatibility check
- direct and serialized/Tauri-shaped traces match
- listeners attach before snapshot reconciliation
- client epoch and attach generation are distinct
- stale results cannot replace current renderer state
- minimal artifact has no Svelte, Poodle, browser, plugin, or GPU dependency
- capabilities authorize protocol access only

## Evidence Required

- generated-source and drift check
- Rust/TypeScript fixture parity
- connection, remount, stale-result, and teardown traces
- package and capability audit
- focused Rust, TypeScript, docs, and Effigy checks

## Stop Conditions

- protocol generation needs product payload knowledge
- Tauri labels become island identity
- capability policy is treated as browser/plugin/render authorization
- framework-neutral lifetime cannot remain independent of Svelte

## Next Task

Execute Card 089. Build the first production mechanism against the checked
kernel and client seam.

## Completion Evidence

- Added Rust-owned versioned connect, snapshot, desired-update, content-size,
  observation, proposal, apply-receipt, and host-destroy protocol shapes.
- Generated checked TypeScript and a Rust-authored golden fixture through the
  `native-content` bindings domain.
- Added framework-neutral `@inflatable-cookie/longhorn-native-content` with direct, serialized,
  and optional narrow Tauri transports.
- Kept renderer client epochs distinct from attach generations. Listener-first
  reconciliation, bounded request correlation, remount, stale-result, and late
  teardown traces pass.
- Kept product content and browser, plugin, GPU, Svelte, and Poodle authority
  outside the package. Capability examples admit protocol access only.
- Focused Rust, TypeScript, generation-drift, package, and Effigy gates pass.
