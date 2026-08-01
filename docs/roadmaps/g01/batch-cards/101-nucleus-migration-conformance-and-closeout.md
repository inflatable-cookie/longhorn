# 101 Nucleus Migration Conformance And Closeout

Status: ready
Owner: Tom
Roadmap: g01.014 batch 4
Governing refs: contracts 001, 003, 004, 009, 012-014, and 017; Cards 094-100
Depends on: Cards 096-100
Auto-start next card: no

## Objective

Prove Nucleus as the first real Surface-free consumer, remove superseded donor
mechanisms, transfer test authority, and close g01.014 with rollback evidence.

## Repository Scope

- Nucleus and Longhorn may change only for conformance, cleanup, and docs.
- Legacy source cleanup requires an exact committed migration receipt and
  explicit operator authority.

## Scope

- clean and legacy-profile startup/restart matrices
- storage, window, layout, renderer, and Browser cross-language traces
- artifact, lock, optional-edge, capability, and peer-runtime inventories
- duplicate mechanism removal and retained product-adapter audit
- migration failure and previous-build rollback drills
- legacy cleanup eligibility without automatic deletion
- authority map, compatibility matrix, docs, and milestone closeout

## Steps

1. Rebuild from exact clean private sources and matching produced artifacts.
2. Run fresh-profile and legacy-profile install/restart matrices.
3. Compare native and renderer traces across every migrated system.
4. Audit optional dependencies, capabilities, peers, and Surface absence.
5. Search for duplicate active mechanisms and classify retained consumer code.
6. Exercise interrupted migration and previous-build rollback.
7. Verify cleanup eligibility without deleting retained source.
8. Update both repos' authority, compatibility, roadmap, and evidence docs.
9. Run full Effigy QA in Longhorn and Nucleus.
10. Close Card 101 and g01.014 only when every authority transfer is exact.

## Acceptance Criteria

- private Longhorn dependencies map to exact clean source commits and matching
  produced artifact identities
- fresh install and `.nucleus` migration both survive restart
- window, project layout, panel, project switch, and Browser behavior passes
- native and renderer traces match the shared fixtures
- no Surface or Surface-transfer dependency, type, state, or command remains
- no superseded storage, window, layout, renderer, or child coordination copy
  remains active
- retained Nucleus code is explicitly product policy or adapter code
- rollback restores one previous authority without dual-write or silent fallback
- source cleanup is only eligible through exact receipt-bound verification
- Longhorn and Nucleus roadmaps, authority maps, logs, and compatibility claims
  agree
- full Effigy QA passes in both repositories

## Evidence Required

- two-profile restart and migration report
- native/renderer conformance traces
- package, lock, capability, Surface-absence, and duplicate-code audits
- rollback and cleanup-eligibility receipts
- retained-policy inventory
- full Longhorn and Nucleus QA
- g01.014 closeout log

## Stop Conditions

- any migrated mechanism still has two active authorities
- rollback loses state or needs a compatibility shim
- cleanup target is not bound to the exact committed migration receipt
- behavior delta lacks operator acceptance
- package or capability graph is broader than the selected composition
- either repository's full QA fails

## Next Task

Return to the g01 front door. Use Nucleus evidence to compile or unblock the
g01.015 Loophole full-hosting migration; do not assume the no-Surface adapter
fits Loophole unchanged.
