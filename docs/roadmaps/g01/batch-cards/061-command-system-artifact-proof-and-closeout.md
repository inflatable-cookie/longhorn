# 061 Command System Artifact Proof And Closeout

Status: complete
Owner: Tom
Roadmap: g01.010 batch 4
Governing refs: contracts 001, 003-006, 010, 012, and 013; research memo 014
Depends on: Card 060
Auto-start next card: no

## Objective

Prove rich Loophole-shaped and minimal Jetstream-shaped command compositions
from produced Rust and TypeScript artifacts, publish composition and migration
guidance, audit the final boundaries, and close g01.010.

## Scope

- produced command, config, Tauri, TypeScript, Svelte, Poodle, and settings
  artifacts
- Loophole-shaped native and Svelte catalogue, context, palette, and keymap
  composition
- Jetstream-shaped global-context keyboard composition
- local renderer and typed-domain execution adapters
- stale availability, focus, conflict, persistence, and teardown traces
- package, feature, peer, capability, payload, authority, and artifact audits
- command-system composition guide
- retained, changed, rejected, deferred, and donor-migration notes
- milestone closeout and full Effigy QA

## Public Behavior

Both proof apps consume produced artifacts and register their own commands,
contexts, presets, availability, and executors. The rich app composes optional
Svelte/Poodle/settings edges. The minimal app resolves none of them.

One command keeps the same semantic identity through palette, shortcut, menu,
and help projection while execution maps to a consumer-owned typed operation.
No proof uses sibling source or modifies a donor repository.

## Out Of Scope

- Loophole or Jetstream repository writes
- public registry publication
- production native accelerator support
- macros or extended triggers
- server-synchronized keymaps
- g01.015 or g01.016 migration completion

## Steps

1. Pack every command-system Rust and TypeScript artifact.
2. Install isolated rich and minimal proof roots.
3. Prove optional dependency absence in the minimal graph.
4. Exercise registry, arguments, availability, keyboard, search, projection,
   mutation, recovery, and execution traces.
5. Prove local and typed-domain executor mappings without generic transport.
6. Verify Poodle public API and settings admission in the rich shape.
7. Audit artifact identity, capabilities, peers, payloads, command routes, and
   authority.
8. Publish composition and later donor-cutover guidance.
9. Record behavior deltas and deferred trigger/macro work.
10. Run full Effigy QA and close Cards 056-061 plus g01.010.

## Acceptance Criteria

- all proofs consume produced artifacts
- no proof resolves sibling source or undeclared optional packages
- minimal Jetstream shape imports no config host, settings, Svelte, Poodle, or
  bridge package
- rich Loophole shape uses one registry and keymap across native and Svelte
  projections
- stale availability and context races reject before product execution
- conflict and persistence failure leave the prior effective keymap intact
- local and typed-domain routes preserve admitted command semantics
- no generic execution endpoint or donor payload exists in shared artifacts
- Poodle usage stays on public controlled APIs
- guidance distinguishes conformance proof from donor cutover
- full Effigy QA passes

## Evidence Required

- artifact identities and isolated install report
- rich/minimal dependency and capability matrix
- cross-language semantic trace
- stale, conflict, persistence, focus, and teardown matrix
- local versus typed-domain execution proof
- payload, authority, execution-bus, peer, and Poodle audits
- composition and migration guide
- milestone closeout log
- full Effigy QA

## Stop Conditions

- a proof resolves sibling source
- a shared artifact contains donor command catalogues or product route logic
- the minimal graph resolves optional UI, settings, config-host, or bridge
  dependencies
- native and Svelte semantics differ for the same fixture
- an execution adapter requires a generic bridge/Tauri command bus
- full QA exposes a contract or package-boundary failure

## Next Task

The g01.011 research gate is complete. Execute Card 062 from the compiled
history runway.
