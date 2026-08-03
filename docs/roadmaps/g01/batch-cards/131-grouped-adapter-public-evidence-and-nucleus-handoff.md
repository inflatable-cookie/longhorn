# 131 Grouped Adapter Public Evidence And Nucleus Handoff

Status: complete
Owner: Tom
Roadmap: g01.019 batch 4
Governing refs: contracts 001, 004, and 012; Cards 128-130
Depends on: Card 130
Auto-start next card: no

## Objective

Close the generic library lane and leave one exact read-only Nucleus resume gate.

## Scope

- public Rust API and crate documentation
- generated API/reference inventory
- isolated package and compatibility proof
- regression proof for ordinary and single-adapter restore
- Longhorn closeout and Nucleus handoff log

## Steps

1. Audit public names, bounds, errors, receipts, and documentation.
2. Run isolated package and Rust 1.85 checks.
3. Run focused and aggregate configuration QA.
4. Record exact unsupported and consumer-owned boundaries.
5. Write the Nucleus resume handoff without editing Nucleus.

## Acceptance Criteria

- public API supports boot-time/offline execution without renderer state
- compatibility evidence includes mixed and SQLite adapters
- ordinary file and explicit single-domain APIs remain proven
- Nucleus handoff names its remaining quiescence and restart work
- Nucleus restore is not claimed complete

## Evidence Required

- API inventory
- package and compatibility receipt
- focused and aggregate QA receipts
- closeout log
- Nucleus consumer handoff

## Stop Conditions

- public API still depends on test-only adapter vocabulary
- aggregate QA exposes an unclassified restore regression
- Nucleus must change before generic conformance can pass

## Next Task

Return to g01.017 Card 070 after Nucleus receives the grouped-restore handoff.

## Evidence

- `crates/longhorn-config/README.md` and the grouped API reference inventory the surface
- focused Rust, Rust 1.85, Clippy, generated-binding, and package-list gates pass
- ordinary file and separate-adapter suites pass with the grouped fixtures
- the Longhorn closeout and Nucleus consumer handoff are indexed in logs
