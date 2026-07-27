# 012 Distribution And Compatibility

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27  
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
- Package names are working names until registry ownership is verified before
  first publication.

## Dependency Discipline

- Pure domain packages cannot depend on Tauri, Svelte, Poodle, or a consumer.
- Optional systems are separate packages; disabling Surfaces, history,
  commands, settings, or service topology removes their dependency graph.
- Adapter packages depend only on the capabilities they adapt.
- Generated TypeScript bindings live with their owning domain package.
- No umbrella package is published in g01.
- Empty placeholder packages are not scaffolded before their implementation
  milestone.

## Consumer Adoption

- First migrations consume published prerelease versions.
- Local path or file dependencies are allowed for proof work but are not a
  merge target.
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

