# 095 Nucleus Prerelease And Cutover Admission

Status: planned; operator, registry, and release gated
Owner: Tom
Roadmap: g01.014 batch 1
Governing refs: contracts 003, 010, 012, and 013; Card 094
Depends on: Card 094
Auto-start next card: no

## Objective

Produce and verify the exact published prerelease graph Nucleus will pin before
the first donor write.

## Repository Scope

- Longhorn: release metadata, artifacts, compatibility evidence, and docs.
- Nucleus: read-only dependency and toolchain compatibility checks.
- External registries: publication only with explicit release authority.

## Scope

- final Cargo and npm registry names
- coordinated prerelease version
- Longhorn packages selected by Cards 096-100
- compatible published Poodle prerelease exposing built-in overlay geometry
  without DOM selectors or host-runtime coupling
- Poodle callback semantics matching
  `../../../architecture/poodle-overlay-geometry-boundary.md`
- Rust 1.85, Tauri 2.11, Svelte 5, and TypeScript compatibility
- produced-artifact installs outside workspace resolution
- exact Nucleus Cargo and renderer pin plan
- rollback artifact availability

## Steps

1. Resolve final registry ownership and coordinated package names.
2. Select one prerelease version and produce every required Rust/npm artifact.
3. Publish only after explicit external release authority.
4. Install the published graph in clean Nucleus-shaped temporary consumers.
5. Verify Rust, Tauri, Svelte, TypeScript, Poodle, protocol, and exact overlay
   intersection compatibility.
6. Audit the exact optional graph, peers, locks, and artifact integrity.
7. Confirm Nucleus g05 acceptance and record the donor cutover decision.
8. Preserve the prior artifact/build set and publish rollback notes.

## Acceptance Criteria

- every selected crate and package exists at one published prerelease version
- Nucleus-shaped clean installs use no path, file, link, or source alias
- one compatible Svelte and Poodle runtime resolves
- Poodle publicly reports built-in overlay geometry needed for exact Browser
  viewport intersection without exposing component internals
- the produced Poodle contract reports immutable viewport snapshots for every
  Popover/Menu surface and removes them on teardown
- no Surface or Surface-transfer artifact enters the graph
- generated Rust/TypeScript protocols match the published artifacts
- exact pins and integrity evidence are recorded before donor mutation
- the previous Nucleus build and legacy source remain available for rollback
- Nucleus g05 project-layout validation is closed or explicitly superseded

## Evidence Required

- registry ownership and publication receipts
- package and lock inventories
- clean Rust and renderer install reports
- compatibility and generated-binding reports
- no-Surface dependency audit
- cutover admission decision

## Stop Conditions

- registry ownership or package names are unresolved
- publication would require ungranted external authority
- Poodle has no compatible published built-in overlay geometry seam
- Nucleus requires a sibling path or unpacked source dependency
- Nucleus g05 still governs unaccepted overlapping behavior
- the produced graph differs from Card 094 fixtures

## Next Task

Run Nucleus g05 Card 003's two operator checks and obtain explicit registry
publication authority. Execute Card 096 only after every admission gate passes.
