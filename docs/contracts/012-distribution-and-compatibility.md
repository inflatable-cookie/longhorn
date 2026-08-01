# 012 Distribution And Compatibility

Status: active compiled boundary
Owner: Tom
Updated: 2026-07-30
Architecture: `../architecture/package-topology.md`

## Boundary

Longhorn ships a coordinated family of capability packages. Consumers import
only the systems they compose. Package-manager choice and application release
policy remain consumer-owned.

## Workspace And Versions

- Cargo and npm workspaces live in one repository.
- Publishable Rust and TypeScript packages use one coordinated Longhorn
  version.
- Rust crates use edition 2024 with MSRV 1.85 until an explicit compatibility
  change.
- TypeScript packages target Svelte 5 and Tauri 2 where those peers apply.
- Svelte, Poodle, and Tauri are peer dependencies of adapters, never hidden
  duplicate runtimes.
- The initial Svelte compatibility floor is `5.38.6`; a package must prove its
  declared upper bound instead of inheriting the newest donor version.
- Private Poodle adapter work uses one exact packable preview artifact. It
  claims no broader compatibility range before a published prerelease exists.
- Package names are working names until registry ownership is verified before
  first publication.

## Dependency Discipline

- Pure domain packages cannot depend on Tauri, Svelte, Poodle, or a consumer.
- Optional systems are separate packages; disabling Surfaces, history,
  commands, settings, or service topology removes their dependency graph.
- Adapter packages depend only on the capabilities they adapt.
- A shared adapter package may expose optional capability subpaths only when
  those capabilities are optional peers and the root entry does not re-export
  them.
- `@longhorn/tauri` is a structural invoke/listen transport. Tauri is its peer;
  no domain package is its mandatory dependency.
- `longhorn-bridge` and `@longhorn/bridge` own only generic session,
  topology, authority, correlation, retry, and ordering contracts. Domain
  payloads stay in their owning packages.
- Service-supervision and production network transports are optional adapter
  edges. A no-service consumer resolves none of them.
- Generated TypeScript bindings live with their owning domain package.
- No umbrella package is published in g01.
- Empty placeholder packages are not scaffolded before their implementation
  milestone.

## Consumer Adoption

- First migrations consume published prerelease versions.
- Local path or file dependencies are allowed for proof work but are not a
  merge target.
- Source aliases, private DOM selectors, and unpacked sibling workspaces are
  not artifact-install or compatibility evidence.
- Consumer lockfiles pin exact resolved versions.
- Protocol compatibility and package compatibility are both tested.
- A breaking serialized contract causes a coordinated major release.
- Deprecations name a replacement and survive at least one compatible release
  unless a security or correctness issue forbids it.

## Release Evidence

A release candidate requires:

- Rust and TypeScript package checks
- generated-binding drift check
- cross-language fixtures
- direct and Tauri adapter conformance
- deterministic serialized-loopback conformance without a production-network
  compatibility claim
- declared consumer compatibility matrix
- install proof from produced artifacts, not workspace source
- migration and rollback notes

## Acceptance

- Nucleus can install window/layout support without Surfaces
- Bovine can install configuration and shell support without layout hosting
- Loophole can compose the full optional hierarchy
- package graph has no upward optional-module edge or peer duplication
- an artifact-install example passes outside workspace resolution
- compatibility failures name the package, protocol, and supported range
- a Surface-free root import does not resolve Surface or Surface-transfer code
- Poodle adapter evidence names the exact preview artifact under test

## Current Poodle Preview Checkpoint

Card 039 is bound to artifact set
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`
from `~/Dev/projects/poodle/.artifacts/g12.016-A698XB/evidence.json`.

The five packages are `@poodle/headless`, `@poodle/styles`,
`@poodle/svelte-tokens`, `@poodle/icons-lucide`, and `@poodle/svelte`, all at
`0.1.0`. The supported Svelte peer range is `>=5.38.6 <6`. Adapter evidence
must verify the set id and individual SHA-256 values recorded in Card 038
before mounting.

Card 041 packs seven Longhorn packages and installs Bovine, Nucleus, and
Loophole proof consumers into isolated roots. Nucleus resolves no Surface
package; Bovine resolves no layout host; Loophole resolves the full optional
graph. Each root contains one Svelte `5.38.6` runtime and the exact five
Poodle tarballs.

Evidence:
`../logs/2026-07/29-three-shape-app-shell-proof-and-closeout.md`.

Card 048 packs the settings TypeScript family and installs isolated Bovine,
Soundcheck, Loophole, and Nucleus consumers against the same exact Poodle set.
Each root contains one Svelte `5.38.6` runtime, no sibling source alias, and
only its declared Longhorn optional systems. Six private Rust crate
inventories also unpack and build offline. Registry-normalized Cargo packages
remain a public release-lane gate while the interdependent crates are private.

Evidence:
`../logs/2026-07/29-settings-composition-proof-and-closeout.md`.

Card 055 packs `@longhorn/core`, `@longhorn/tauri`, and
`@longhorn/bridge`, then installs Bovine, Jetstream, Soundcheck, Nucleus, and
Loophole-shaped consumers in isolated roots. Every consumer resolves version
`0.1.0`, one Tauri API `2.10.1` peer, exact declared bridge subpaths, and no
workspace or sibling source alias. Separate offline Rust consumers prove a
query-only graph without supervision/Tauri and a selected full-host graph.

The private Rust evidence uses `cargo package --list` plus exact source
archives. Registry-normalized Cargo packaging remains a release-lane gate.

Evidence:
`../logs/2026-07/30-bridge-topology-artifact-proof-and-closeout.md`.

## Native-content Artifact Checkpoint

Card 093 inventories five private Rust source artifacts after successful
`cargo package --list`, then compiles pure, child-view, isolated-window, and
backing-surface consumers offline on Rust 1.85. Registry-normalized Cargo
packages remain a release-lane gate.

It packs `@longhorn/core`, `@longhorn/native-content`, and
`@longhorn/native-content-svelte` and installs isolated Nucleus, Soundcheck,
and Jetstream shapes. Each lock resolves version `0.1.0`, no workspace or
sibling source alias, only its selected mechanism, and one Svelte/Poodle
runtime where the Svelte adapter is selected. The exact Card 039 Poodle
artifact set is reverified before mounted compilation.

Rust-produced and renderer traces match across all three mechanism semantics.
This is artifact-install evidence, not a registry-publication or donor-cutover
claim.

Evidence:
`../logs/2026-08/01-native-content-artifact-proof-and-closeout.md`.
