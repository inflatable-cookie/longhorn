# 127 Private 0.1 Compatibility Candidate And Closeout

Status: complete
Owner: Tom
Roadmap: g01.016 batch 5
Governing refs: contracts 001, 003, 010, and 012-013; Cards 113-126
Depends on: Card 126
Auto-start next card: no

## Objective

Prove a coordinated, deterministic private `0.1.0` Longhorn candidate across
all supported packages and consumers, then close g01.016 without registry,
tag, or hosted-release mutation.

## Repository Scope

- Longhorn: version metadata, local artifacts, checksums, compatibility
  evidence, changelog, docs, and closeout.
- Nucleus, Loophole, Soundcheck, Split-shell, Jetstream, and Poodle: read-only exact
  receipt and isolated-consumer checks.
- Registries, Git tags, hosted releases, and consumer release channels: no writes.

## Scope

- coordinated Rust/TypeScript version inventory
- deterministic package contents, checksums, generated bindings, and source mapping
- Rust 1.85, Tauri 2.11, Svelte 5, TypeScript, Poodle, platform, and protocol matrix
- minimal, Nucleus, Loophole, Soundcheck, Split-shell, Jetstream, and optional-server installs
- public API, migration, rollback, compatibility, and upgrade notes
- package-name/registry-ownership gap and deferred publication statement

## Steps

1. Freeze exact clean Longhorn and Poodle sources.
2. Verify every publishable package reports coordinated `0.1.0` metadata.
3. Produce deterministic local Rust inventories and npm archives with checksums.
4. Run generated-binding, direct/Tauri, serialized-loopback, and package checks.
5. Install every declared consumer graph from the produced artifacts.
6. Run read-only Effigy release gates and classify every unmet publication-only gate.
7. Audit changelog, API docs, compatibility matrix, migration, and rollback notes.
8. Close g01.016 and retain registry ownership/publication as an explicit later gate.

## Acceptance Criteria

- all selected Rust and TypeScript packages report coordinated `0.1.0`
- produced contents and checksums are reproducible from one clean source
- all declared consumer and greenfield graphs install without sibling resolution
- protocol and package compatibility failures name exact supported ranges
- MSRV and native platform claims are backed by compatible locks and packaged evidence
- no optional edge enters a consumer that did not select it
- no registry write, package publication, tag, GitHub release, or consumer release occurs
- registry ownership and public package names remain visibly unresolved

## Stop Conditions

- a produced artifact differs between identical clean builds
- an isolated consumer needs workspace or sibling resolution
- a declared compatibility claim lacks evidence
- a release gate can pass only by weakening package or protocol checks
- any action would publish, tag, or create a hosted release

## Evidence

- `fixtures/release/card127/private-0-1-candidate-v1.json` records the exact
  source, tool, package, checksum, peer, protocol, platform, and seven-graph
  compatibility receipt.
- `docs/reference/private-0-1-candidate.md` is the human compatibility matrix.
- 17 Longhorn TypeScript packages, five Poodle packages, and 36 Rust packages
  reproduce across two independent production passes.
- Minimal, Nucleus, Loophole, Soundcheck, Split-shell, Jetstream, and optional-
  server roots install outside sibling resolution and pass locked Rust 1.85.
- All 12 generated protocol families are current.
- Both configured read-only Effigy release gates pass. No publication, tag,
  hosted release, or consumer release occurs.
- `docs/logs/2026-08/02-private-0-1-compatibility-candidate-and-closeout.md`
  records the closeout.

## Next Task

Execute Card 070. Implement the optional pure fork-tree identity, topology,
and branch authority without changing linear artifacts.
