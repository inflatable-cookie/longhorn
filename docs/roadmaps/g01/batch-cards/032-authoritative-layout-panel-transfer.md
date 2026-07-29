# 032 Authoritative Layout Panel Transfer

Status: ready
Owner: Tom
Roadmap: g01.006 batch 4
Governing refs: contracts 001, 004, 009-011, and 014; research memo 010
Depends on: Card 031 and completed Cards 024-025
Auto-start next card: no

## Objective

Bind transfer sessions to the existing layout mutation and persistence
authority for exact same-document cross-window panel moves.

## Scope

- panel transfer source admission
- direct-window and Surface-container host-binding adapters
- fresh source and target re-resolution
- same registered layout-domain requirement
- target capability, region, movability, and instance-policy checks
- existing expected-revision `MovePanel` commit
- authoritative completion and abort receipts
- cross-document rejection before mutation
- Loophole-shaped and Nucleus-shaped conformance

## Public Behavior

The host never trusts the session's old source or target snapshot. Commit
reloads one registered layout document, resolves both containers, and invokes
the existing move command under coordinator authority.

Move is the only supported panel operation. Cross-document and copy requests
fail before publication. Every terminal outcome consumes the transfer session.

## Out Of Scope

- cross-document transactions
- panel copy
- automatic new-window creation
- Surface lifecycle mutation
- Tauri, TypeScript, Svelte, Poodle, or donor changes

## Steps

1. Admit movable panel instances from fresh layout authority.
2. Record source container, region, domain, and revision ids only.
3. Resolve direct-window targets through external host bindings.
4. Resolve Surface targets through the optional binding adapter.
5. Recheck current windows, containers, regions, revisions, and capability.
6. Reject cross-document and copy operations before mutation.
7. Commit one existing `MovePanel` through `longhorn-layout-config`.
8. Consume the session and return authoritative target evidence.
9. Prove exact unchanged source on every abort.
10. Run both composition shapes through one transfer matrix.

## Acceptance Criteria

- no product attachment enters transfer state
- source disappearance or movement aborts
- stale source or target revision aborts
- ineligible target or instance-policy failure aborts
- target-window disappearance aborts
- same-document move commits one layout revision
- cross-document move publishes no bytes
- copy is explicitly unsupported
- direct-window fixture contains no Surface type
- Surface fixture retains current container binding
- success returns the exact authoritative layout snapshot

## Evidence Required

- source admission and target re-resolution matrix
- same-document publication receipt
- cross-document zero-publication proof
- stale, disappeared, ineligible, and replay fixtures
- two-shape conformance fixture
- exact failure invariance report
- Rust 1.85 and full Effigy QA

## Stop Conditions

- commit needs a new duplicate mutation
- cross-document atomicity is required for the proof
- host binding enters layout persistence
- transfer bypasses expected revision
- session state becomes mutation authority

## Next Task

Card 033 remains planned until panel-transfer authority and exact abort
invariance close.
