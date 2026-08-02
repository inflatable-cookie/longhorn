# 114 Secondary-consumer Private Artifact Admission

Status: planned
Owner: Tom
Roadmap: g01.016 batch 1
Governing refs: contracts 003, 010, 012, 013, and 017; Card 113
Depends on: Card 113
Auto-start next card: no

## Objective

Prove the exact private Longhorn and Poodle graphs selected by Soundcheck,
Bovine, and Jetstream before admitting consumer writes. Do not publish.

## Repository Scope

- Longhorn: package metadata, proof consumers, artifacts, evidence, and docs.
- Consumers and Poodle: read-only compatibility checks.
- Temporary isolated install roots: disposable proof output only.
- Package registries: no writes.

## Scope

- exact clean Longhorn and Poodle source receipts
- Rust 1.85, Tauri 2.11, Svelte 5, TypeScript, and Bun compatibility
- selected private Rust and TypeScript package graphs per consumer
- exact Poodle packed artifact set where UI adapters are selected
- isolated installs with no workspace, sibling, path, or source-alias resolution
- generated binding, peer, capability, protocol, and optional-edge audits
- explicit sibling-source development maps and matching artifact identities

## Steps

1. Refresh exact selected Longhorn and Poodle commits.
2. Pack or inventory every selected Rust and TypeScript artifact.
3. Build isolated Soundcheck-, Bovine-, and Jetstream-shaped consumers.
4. Verify exact toolchain, peer, lock, and generated-protocol compatibility.
5. Audit each graph for selected and forbidden optional packages.
6. Record explicit private-development source maps and artifact digests.
7. Preserve previous consumer locks and source receipts for rollback.
8. Admit only the first bounded Soundcheck slice.

## Acceptance Criteria

- every selected artifact maps to one exact clean source commit
- isolated consumers resolve no sibling or workspace source
- one compatible Svelte/Poodle runtime resolves where selected
- Bovine resolves no layout, Surface, bridge, operation, history, or native-content edge
- Soundcheck resolves no Surface, transfer, history, command, or bridge edge
- Jetstream resolves no config, settings, Surface, transfer, history, or operation edge
- native-content mechanisms remain independently selectable
- no package-manager publication, release tag, or hosted release occurs

## Stop Conditions

- a clean proof needs a source alias or unpacked sibling
- selected artifacts differ from their receipts
- an optional package enters the wrong consumer graph
- a consumer toolchain falls outside proven compatibility
- Bovine overlap clearance is required before the first Soundcheck slice

## Next Task

Execute Card 115's Soundcheck storage and window cutover from the admitted
private graph.
