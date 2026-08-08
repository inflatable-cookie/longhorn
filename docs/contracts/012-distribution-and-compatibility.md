# 012 Distribution And Compatibility

Status: active compiled boundary
Owner: Tom
Updated: 2026-08-04
Architecture: `../architecture/package-topology.md`

## Boundary

Longhorn ships a coordinated family of capability packages. Consumers import
only the systems they compose. Package-manager choice and application release
policy remain consumer-owned.

## Workspace And Versions

- Cargo and npm workspaces live in one repository.
- Publishable Rust and TypeScript packages use one coordinated Longhorn
  version.
- Rust crates use edition 2024 with MSRV 1.95 until an explicit compatibility
  change.
- TypeScript packages target Svelte 5 and Tauri 2 where those peers apply.
- Svelte, Poodle, and Tauri are peer dependencies of adapters, never hidden
  duplicate runtimes.
- The initial Svelte compatibility floor is `5.38.6`; a package must prove its
  declared upper bound instead of inheriting the newest donor version.
- Private Poodle adapter work uses one exact source commit and packable preview
  artifact. It claims no broader compatibility range before the release lane
  proves one.
- Package names are working names until registry ownership is verified before
  first publication.

## Dependency Discipline

- Pure domain packages cannot depend on Tauri, Svelte, Poodle, or a consumer.
- **Rust:** optional systems are separate crates, and disabling Surfaces,
  history, commands, settings, or service topology removes their dependency
  graph. This is measured, not asserted: finch compiles 8 crates of 41,
  soundcheck 17, nucleus and loophole 24. No consumer takes more than 59%.
- **TypeScript:** the same claim does not hold and must not be made. Every
  domain package depends on `core` and nothing else; `layout` and `commands`
  depend on nothing at all, and no third-party dependency differs between
  them. Removing a domain removes that domain — there is no tree beneath it
  to prune, and tree-shaking already keeps unused exports out of an
  application bundle.
- TypeScript packages are therefore grouped by **peer requirement**, which is
  the only axis on which they genuinely differ, not by domain. Card 164
  collapsed eighteen packages into three on that axis:
  `@inflatable-cookie/longhorn` (no peers), `-poodle-svelte` (svelte, Poodle,
  longhorn), and `-tauri` (`@tauri-apps/api`, longhorn). Every domain is a
  subpath; nothing that resolved before stopped resolving.
- One package per peer tier also removes a failure mode the split created:
  independently installed lockstep packages could skew, and
  `longhorn-svelte` alone peered on four separate longhorn packages. Four
  skew opportunities became zero.
- Adapter packages depend only on the capabilities they adapt.
- A shared adapter package may expose optional capability subpaths only when
  those capabilities are optional peers and the root entry does not re-export
  them.
- `@inflatable-cookie/longhorn-tauri` is a structural invoke/listen transport. Tauri is its peer;
  no domain package is its mandatory dependency.
- `longhorn-bridge` and `@inflatable-cookie/longhorn/bridge` own only generic session,
  topology, authority, correlation, retry, and ordering contracts. Domain
  payloads stay in their owning packages.
- Service-supervision and production network transports are optional adapter
  edges. A no-service consumer resolves none of them.
- Generated TypeScript bindings live with their owning domain package.
- No umbrella package is published in g01.
- Empty placeholder packages are not scaffolded before their implementation
  milestone.

## Consumer Adoption

- Package-manager publication is deferred during the private development and
  first-migration phase. It does not gate donor cutover.
- Private consumer repos may merge explicit sibling path or `file:` sources.
  Each adopted graph records the exact clean Longhorn and Poodle commits.
- Moving branch references and unrecorded dirty sources are not valid pins.
- Every source-linked graph also passes an isolated install from produced
  artifacts. Source aliases, private DOM selectors, and unpacked sibling
  workspaces are not artifact-install or compatibility evidence.
- Registry names, normalized Cargo packages, versions, and public compatibility
  ranges remain a later release-lane decision.
- Consumer lockfiles pin exact resolved versions.
- An MSRV claim uses an exact compatible lock. A fresh unconstrained resolver
  result is not MSRV evidence when transitive dependencies may raise their
  toolchain floor.
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

- Nucleus composes window/layout support without importing Surfaces, and
  Bovine composes configuration and shell support without importing layout
  hosting. Stated as imports rather than installs: Card 164 put every domain
  in one package, so a consumer resolves them all and tree-shaking is what
  keeps the unused ones out of a bundle. The Rust tier keeps the install
  claim, where the split is still real.
- Loophole can compose the full optional hierarchy
- package graph has no upward optional-module edge or peer duplication
- an artifact-install example passes outside workspace resolution
- compatibility failures name the package, protocol, and supported range
- a Surface-free root import does not resolve Surface or Surface-transfer code
- Poodle adapter evidence names the exact preview artifact under test

## Current Poodle Preview Checkpoint

**Superseded 2026-08-08 by Poodle Card 020.** Poodle collapsed its six
publish-intent packages into three grouped by framework binding, so the set
below is now two tarballs rather than five:

- artifact set
  `ec626ac7c66cfd45249690270195e7be74e34fe96940e32a938852a41ed7bba8`
  from `~/Dev/projects/poodle/.artifacts/svelte-pack-install-V2Wu2n/`
- `@inflatable-cookie/poodle-core` — `ac13b2ef31c84b11…`, 291,471 bytes
- `@inflatable-cookie/poodle-svelte` — `8a845d0fb7279ef5…`, 200,091 bytes

`poodle-core` carries the former `poodle-headless`, `poodle-styles`,
`poodle-svelte-tokens` and `poodle-icons-lucide` as the subpaths `.`,
`./styles/*`, `./tokens/*` and `./icons/*`. Longhorn pins both tarballs.

The paragraphs below record the sets each card was bound to at the time and
keep their original package names deliberately.

Card 039 is bound to artifact set
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`
from `~/Dev/projects/poodle/.artifacts/g12.016-A698XB/evidence.json`.

The five packages are `@inflatable-cookie/poodle-headless`, `@inflatable-cookie/poodle-styles`,
`@inflatable-cookie/poodle-svelte-tokens`, `@inflatable-cookie/poodle-icons-lucide`, and `@inflatable-cookie/poodle-svelte`, all at
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

Card 101 repeats the Nucleus graph against exact current clean sources:
Longhorn `4f09e1c8`, Poodle `208532f0`, Poodle artifact set `25083fe0`, and
Nucleus `40a8314d`. The isolated renderer resolves one Svelte/Poodle runtime
and no Surface package. The Rust graph contains no Surface package. Registry
publication remains deferred.

Evidence:
`../logs/2026-08/01-nucleus-migration-conformance-closeout.md`.

Card 055 packs `@inflatable-cookie/longhorn/core`, `@inflatable-cookie/longhorn-tauri`, and
`@inflatable-cookie/longhorn/bridge`, then installs Bovine, Jetstream, Soundcheck, Nucleus, and
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
backing-surface consumers offline on Rust 1.95. Registry-normalized Cargo
packages remain a release-lane gate.

It packs `@inflatable-cookie/longhorn/core`, `@inflatable-cookie/longhorn/native-content`, and
`@inflatable-cookie/longhorn-poodle-svelte/native-content` and installs isolated Nucleus, Soundcheck,
and Jetstream shapes. Each lock resolves version `0.1.0`, no workspace or
sibling source alias, only its selected mechanism, and one Svelte/Poodle
runtime where the Svelte adapter is selected. The exact Card 039 Poodle
artifact set is reverified before mounted compilation.

Rust-produced and renderer traces match across all three mechanism semantics.
This is artifact-install evidence, not a registry-publication or donor-cutover
claim.

## Private Migration Distribution Decision

Nucleus and the other development apps may consume Longhorn and Poodle from
explicit sibling sources while both libraries remain private. The source
receipt names the exact clean commit. A separate clean temporary consumer must
install the matching packed npm archives and Rust source artifacts without
workspace or sibling resolution before the dependency graph is admitted.

Package-manager publication, registry ownership, normalized Cargo archives,
and public version-range promises are deferred to g01.016 release preparation.
They are not gates for g01.014 or g01.015 donor migrations.

Evidence:
`../logs/2026-08/01-native-content-artifact-proof-and-closeout.md`.

## Nucleus Private Migration Checkpoint

Card 095 installs eight selected Longhorn TypeScript packages plus the exact
five-package Poodle g12.018 artifact set in a clean Nucleus-shaped renderer.
It compiles the public Poodle overlay geometry seam on Svelte 5.56.8,
TypeScript 6.0.3, and Tauri API 2.11.1. No Surface package resolves.

The Rust consumer compiles eleven selected Longhorn packages with Tauri
2.11.5 on Rust 1.95 from an exact offline lock. An unlocked Tauri 2.11
resolution can select transitive crates requiring newer Rust, so it does not
prove Longhorn's MSRV. The selector emits the evolved lock digest with the
artifact and source receipts.

The operator accepted both Nucleus layout checks after the dependency proof.
Card 095 therefore admits bounded donor writes and promotes Card 096. This
makes no registry, publication, or public compatibility-range claim.

Evidence:
`../logs/2026-08/01-nucleus-private-artifact-admission.md`.
