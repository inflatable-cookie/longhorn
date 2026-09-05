# 012 Distribution And Compatibility

Status: active compiled boundary
Owner: Tom
Updated: 2026-08-24
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
- Poodle adapter work pins one exact published Poodle version. The preview era
  -- one exact source commit plus a packable artifact, claiming no range --
  ended when Poodle released publicly; see the checkpoint below.
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

- Package-manager publication is scoped and queued, no longer deferred on
  names: the `@inflatable-cookie` scope was claimed on 2026-08-09, the three
  TypeScript packages are publishable (`publishConfig.access: public`), and
  the v0.1.0 tag waited on a Poodle release carrying the components the
  adapter imports (g02.014). Public Poodle 0.2.2 carries them, so that
  precondition is met and the linked-Poodle exemption is gone (Card 218 step
  5). Rust crates stay `publish = false`; consumers take them by git tag.
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
  Split-shell composes configuration and shell support without importing layout
  hosting. Stated as imports rather than installs: Card 164 put every domain
  in one package, so a consumer resolves them all and tree-shaking is what
  keeps the unused ones out of a bundle. The Rust tier keeps the install
  claim, where the split is still real.
- Loophole can compose the full optional hierarchy
- package graph has no upward optional-module edge or peer duplication
- an artifact-install example passes outside workspace resolution
- compatibility failures name the package, protocol, and supported range
- a Surface-free root import does not resolve Surface or Surface-transfer code
- Poodle adapter evidence names the exact published Poodle release under test

## Current Poodle Checkpoint

**Public registry, exact 0.3.0, adopted 2026-09-05 by g16.109.** There is no
packed preview artifact any more. Longhorn resolves Poodle the way any
consumer does.

- npm: `@inflatable-cookie/poodle-core@0.3.0` and
  `@inflatable-cookie/poodle-svelte@0.3.0`, both `latest` on the public
  registry. Exact, not a range: the adapter's peer is exact, and a range on
  either side would let the two skew.
- Rust: git tag `v0.3.0` at `85609d941a208ff2f854e9f7c0e457089cc77d0e`. One
  tag for `poodle-specs` and for every direct prototype Poodle crate --
  two sources for one crate are two incompatible types.
- `gpui` resolves from crates.io at `0.2.2` on both sides of the graph, the
  consumer's direct dependency and Poodle's transitive one. Poodle 0.2.1
  exposed a fork identity here and is not adoptable; 0.2.0 and 0.2.1 are
  both skipped.
- React remains source-only and has no npm package.

`poodle-core` carries the former `poodle-headless`, `poodle-styles`,
`poodle-svelte-tokens` and `poodle-icons-lucide` as the subpaths `.`,
`./styles/*`, `./tokens/*` and `./icons/*`.

### Superseded preview checkpoints

Kept as the record of what the pin was, not as anything still in force.

**Superseded 2026-08-08 by Poodle Card 020.** Poodle collapsed its six
publish-intent packages into three grouped by framework binding, so the set
below was two tarballs rather than five:

- artifact set
  `ec626ac7c66cfd45249690270195e7be74e34fe96940e32a938852a41ed7bba8`
  from `~/Dev/projects/poodle/.artifacts/svelte-pack-install-V2Wu2n/`
- `@inflatable-cookie/poodle-core` — `ac13b2ef31c84b11…`, 291,471 bytes
- `@inflatable-cookie/poodle-svelte` — `8a845d0fb7279ef5…`, 200,091 bytes

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

Card 041 packs seven Longhorn packages and installs Split-shell, Nucleus, and
Loophole proof consumers into isolated roots. Nucleus resolves no Surface
package; Split-shell resolves no layout host; Loophole resolves the full optional
graph. Each root contains one Svelte `5.38.6` runtime and the exact five
Poodle tarballs.

Evidence:
`../logs/2026-07/29-three-shape-app-shell-proof-and-closeout.md`.

Card 048 packs the settings TypeScript family and installs isolated Split-shell,
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
`@inflatable-cookie/longhorn/bridge`, then installs Split-shell, Jetstream, Soundcheck, Nucleus, and
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
