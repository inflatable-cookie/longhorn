# 095 Nucleus Private Artifact And Cutover Admission

Status: active; artifact proof passes; operator-held
Owner: Tom
Roadmap: g01.014 batch 1
Governing refs: contracts 003, 010, 012, and 013; Card 094
Depends on: Card 094
Auto-start next card: no

## Objective

Produce and verify the exact private Longhorn and Poodle graph Nucleus will
consume before the first donor write. Do not publish to package managers.

## Repository Scope

- Longhorn: private package metadata, artifacts, compatibility evidence, and
  docs.
- Nucleus: read-only dependency and toolchain compatibility checks.
- Poodle: read-only use of the exact g12.018 source and artifact evidence.
- External registries: out of scope.

## Scope

- exact Longhorn and Poodle source commits
- private source-link layout for the Nucleus development checkout
- Longhorn packages selected by Cards 096-100
- Poodle g12.018 built-in overlay geometry contract and packed artifact
- Rust 1.85, Tauri 2.11, Svelte 5, and TypeScript compatibility
- produced-artifact installs outside workspace resolution
- exact Nucleus Cargo and renderer dependency plan
- artifact digests, source receipts, and rollback availability

Nucleus already consumes private Poodle packages through sibling `file:`
sources. The Longhorn migration may use the same explicit private-development
posture. Source links are dependency inputs, not compatibility evidence. The
clean proof must install produced archives without sibling aliases.

## Steps

1. [x] Freeze exact clean Longhorn and Poodle source commits.
2. [x] Select the private package graph and produce every required Rust and npm
   artifact.
3. [x] Install the produced graph in clean Nucleus-shaped temporary consumers.
4. [x] Verify Rust, Tauri, Svelte, TypeScript, Poodle, protocol, and exact overlay
   intersection compatibility.
5. [x] Record the explicit Nucleus sibling-source dependency map and matching
   artifact identities.
6. [x] Audit the exact optional graph, peers, locks, source receipts, and artifact
   integrity.
7. [ ] Close Nucleus g05 Card 003's remaining new-project operator check and record
   donor-write admission.
8. [x] Preserve the prior source/build set and write rollback notes.

## Current Evidence

`effigy proof:nucleus-private-artifacts` returns
`pass_with_operator_gate`. It installs eight Longhorn TypeScript packages and
the exact five-package Poodle g12.018 set in a clean Nucleus-shaped renderer,
then compiles eleven selected Longhorn Rust packages on Rust 1.85 with Tauri
2.11.5 from an exact compatible lock. Neither Surface package enters either
consumer graph.

The proof records exact source receipts, artifact SHA-256 identities, the
private sibling-source plan, generated-binding checks, public Poodle geometry
compilation, and the remaining operator gate. Nucleus and Poodle stay clean
and read-only. Package-manager publication remains out of scope.

## Acceptance Criteria

- Longhorn and Poodle remain private and unpublished to package managers
- every selected package maps to one exact clean source commit and produced
  artifact identity
- Nucleus-shaped clean installs use no path, file, link, or source alias
- private Nucleus development dependencies use only explicit sibling sources
  covered by the recorded commits
- one compatible Svelte and Poodle runtime resolves
- Poodle reports built-in overlay geometry without exposing component
  internals
- the Poodle contract reports immutable viewport snapshots for every
  Popover/Menu surface and removes them on teardown
- no Surface or Surface-transfer package enters the graph
- generated Rust/TypeScript protocols match the produced artifacts
- the previous Nucleus build and legacy source remain available for rollback
- both Nucleus g05 project-layout operator checks are recorded

## Evidence Required

- exact source commit and clean-worktree receipts
- private dependency and package inventories
- produced artifact names and SHA-256 digests
- clean Rust and renderer install reports
- compatibility and generated-binding reports
- no-Surface dependency audit
- cutover admission decision

## Stop Conditions

- a selected source commit is dirty, moving, or unavailable
- a clean proof requires a sibling path or unpacked source dependency
- produced artifacts differ from their recorded source commits
- Poodle geometry semantics differ from Card 094's intended intersection policy
- Nucleus g05's new-project check remains unaccepted at donor-write time
- the produced graph differs from Card 094 fixtures

## Next Task

Record the remaining Nucleus g05 check: a previously unseen project opens with
Agent Chat only. Then admit donor writes, complete this card, and promote Card
096. Do not publish to package managers.
