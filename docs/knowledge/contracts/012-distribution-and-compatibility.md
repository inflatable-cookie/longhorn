# 012 Distribution And Compatibility

Status: active compiled boundary
Owner: Tom
Updated: 2026-10-07
Architecture: `../architecture/package-topology.md`

## Boundary

Longhorn ships a coordinated family of capability packages. Consumers import
only the systems they compose. Package-manager choice and application release
policy remain consumer-owned.

## Workspace And Versions

- Cargo and npm workspaces live in one repository.
- Publishable Rust and TypeScript packages use one coordinated Longhorn
  version.
- Rust crates use edition 2024 with MSRV 1.97.1 until an explicit compatibility
  change.
- TypeScript packages target Svelte 5 and Tauri 2 where those peers apply.
- Svelte, Poodle, and Tauri are peer dependencies of adapters, never hidden
  duplicate runtimes.
- Svelte peer range: `>=5.56.8 <6` from the release after `0.2.2`, matching
  Poodle's own Svelte peer (operator ruling 2026-09-26). Longhorn moves its
  range to meet Poodle, not the reverse: prefer newer releases and never ask
  Poodle to lower its floor. `0.2.2` and earlier declare `>=5.38.6 <=5.56.8`,
  which meets Poodle only at `5.56.8`.
- From `0.3.0`, `longhorn-poodle-svelte` declares Poodle (`poodle-core`,
  `poodle-svelte`) as optional peers with a range within one Poodle minor
  line, currently `>=0.4.16 <0.5` (floor raised 2026-11-26). Below 1.0 a
  Poodle minor is the breaking step, so compatible Poodle patches do not require a
  Longhorn release and consumers keep one Poodle copy. Longhorn's own dev
  and proof installs pin one exact Poodle release, and the proofs cover the
  range ends. `0.2.2` and earlier peer on one exact Poodle version. The
  preview era -- one exact source commit plus a packable artifact, claiming
  no range -- ended when Poodle released publicly; see the checkpoint below.
- Internal npm dependencies and adapter peers pin the exact Longhorn version,
  never `workspace:*`: consumer `file:` installs resolve these packages
  outside the workspace, where `workspace:*` cannot resolve (decided
  2026-08-03). `effigy release:bump` moves them. The adapters' peer on `@inflatable-cookie/longhorn`
  stays exact, not a range (operator reconfirmed 2026-09-27). The three
  packages release together and are tested as one set, so a range would add
  an untested core/adapter pairing and save no release. Revisit only if the
  core and adapters get separate release cadences, together with a check of
  older adapters against the newer core.
- Third-party Rust dependencies use ranges, not `=` pins, from `0.3.0`
  (operator ruling 2026-09-27). Consumers take Longhorn crates by git tag,
  so an exact pin fixes every consumer's version; `rusqlite` links SQLite,
  which one application graph can hold only once. Longhorn's own
  `Cargo.lock` is the tested set, and every dependency refresh reruns the
  byte-stable backup, snapshot and archive suites. The exception is `ts-rs`,
  which stays exact: it reaches consumers only through the opt-in `bindings`
  feature, and its output shape is part of the checked binding contract.
- Generated Tauri `gen/schemas` stay committed (decided 2026-08-14): the
  example capability files reference `../gen/schemas/desktop-schema.json`
  through `$schema`.
- Package names are fixed once published. The `@inflatable-cookie` scope was
  claimed on 2026-08-09, and the three TypeScript packages publish from
  `0.1.0` (g02.014); consumers migrate to the published versions at their next
  uptake.

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
  the only axis on which they genuinely differ, not by domain. The package consolidation
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

- Package-manager publication happens at `0.1.0`: the `@inflatable-cookie`
  scope was claimed on 2026-08-09, the three TypeScript packages are
  publishable (`publishConfig.access: public`), and Poodle is published at
  `0.4.2` (g02.039), so the adapter's precondition is met and the linked-Poodle
  exemption is gone. Rust crates stay `publish = false`;
  consumers take them by git tag.
- Private consumer repos may merge explicit sibling path or `file:` sources.
  Each adopted graph records the exact clean Longhorn and Poodle commits.
- Moving branch references and unrecorded dirty sources are not valid pins.
- Every source-linked graph also passes an isolated install from produced
  artifacts. Source aliases, private DOM selectors, and unpacked sibling
  workspaces are not artifact-install or compatibility evidence.
- Isolated proof installs use a cache per stage. Exact consumer pins remain
  exact; other third-party resolutions use the root Bun lock. A staged lock
  with a third-party version newer than the root lock fails the proof. The
  pack typecheck's newest-admitted peer pass deliberately resolves Svelte and
  Poodle ranges; it reports the versions it tested.
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
  the minimal-shell proof composes configuration and shell support without importing layout
  hosting. The consolidated package puts domains behind export subpaths. A
  consumer resolves the package, and tree-shaking is what
  keeps the unused ones out of a bundle. The Rust tier keeps the install
  claim, where the split is still real.
- Loophole can compose the full optional hierarchy
- package graph has no upward optional-module edge or peer duplication
- an artifact-install example passes outside workspace resolution
- compatibility failures name the package, protocol, and supported range
- a Surface-free root import does not resolve Surface or Surface-transfer code
- Poodle adapter evidence names the exact published Poodle release under test

## Current Poodle Checkpoint

**Longhorn pins Poodle npm packages and the coordinated native Rust tag to
0.4.16.** There is no packed preview artifact any more. Longhorn resolves Poodle the way any
consumer does.

- npm: `@inflatable-cookie/poodle-core@0.4.16` and
  `@inflatable-cookie/poodle-svelte@0.4.16`, from the public registry. Longhorn's dev and proof installs pin one
  release, while the checkout declares the `>=0.4.16 <0.5` peer range
  (floor raised 2026-11-26, Workspace And Versions above).
- Rust: git tag `v0.4.16` at `114c7b5a0bf204b9ea86cccccf7164cb38470586`. One
  tag for `poodle-specs` and for every direct prototype Poodle crate --
  two sources for one crate are two incompatible types.
- GPUI resolves as `gpui-unofficial =1.22.0` on both sides of the prototype
  graph, the consumer's direct dependency and Poodle's transitive backend
  dependency. The manifest aliases it as `gpui` for source compatibility.
- 0.4.4 added a Text/Code `wrap` option and IconButton `data-*` forwarding;
  its peers are unchanged (`svelte >=5.56.8 <6`, `marked ^18.0.9`).
  `0.4.3` was never published.
- React remains source-only and has no npm package.

`poodle-core` carries the former `poodle-headless`, `poodle-styles`,
`poodle-svelte-tokens` and `poodle-icons-lucide` as the subpaths `.`,
`./styles/*`, `./tokens/*` and `./icons/*`.

### Superseded preview checkpoints

Kept as the record of what the pin was, not as anything still in force.

**Superseded 2026-08-08 by Poodle package consolidation.** Poodle collapsed its six
publish-intent packages into three grouped by framework binding, so the set
below was two tarballs rather than five:

- artifact set
  `ec626ac7c66cfd45249690270195e7be74e34fe96940e32a938852a41ed7bba8`
  from `~/Dev/projects/poodle/.artifacts/svelte-pack-install-V2Wu2n/`
- `@inflatable-cookie/poodle-core` — `ac13b2ef31c84b11…`, 291,471 bytes
- `@inflatable-cookie/poodle-svelte` — `8a845d0fb7279ef5…`, 200,091 bytes

The historical proof receipts below retain the original package names.

The five-package adapter proof used artifact set
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`
from `~/Dev/projects/poodle/.artifacts/g12.016-A698XB/evidence.json`.

The five packages are `@inflatable-cookie/poodle-headless`, `@inflatable-cookie/poodle-styles`,
`@inflatable-cookie/poodle-svelte-tokens`, `@inflatable-cookie/poodle-icons-lucide`, and `@inflatable-cookie/poodle-svelte`, all at
`0.1.0`. The supported Svelte peer range is `>=5.38.6 <6`. Adapter evidence
must verify the set id and individual SHA-256 values in the five-package preview receipt
before mounting.

The private shell proof packed seven Longhorn packages and installed a minimal consumer
(retired 2026-10-22), Nucleus and Loophole proof consumers into isolated roots. Nucleus resolves no Surface
package; a minimal consumer resolves no layout host; Loophole resolves the full optional
graph. Each root contains one Svelte `5.38.6` runtime and the exact five
Poodle tarballs.

Evidence:
the retired logs (Git history).

The private settings proof packed the settings TypeScript family and installed isolated minimal
(retired 2026-10-22), Soundcheck, Loophole and Nucleus consumers against the same exact Poodle set.
Each root contains one Svelte `5.38.6` runtime, no sibling source alias, and
only its declared Longhorn optional systems. Six private Rust crate
inventories also unpack and build offline. Registry-normalized Cargo packages
remain a public release-lane gate while the interdependent crates are private.

Evidence:
the retired logs (Git history).

Final Nucleus conformance repeated the Nucleus graph against exact current clean sources:
Longhorn `4f09e1c8`, Poodle `208532f0`, Poodle artifact set `25083fe0`, and
Nucleus `40a8314d`. The isolated renderer resolves one Svelte/Poodle runtime
and no Surface package. The Rust graph contains no Surface package. Registry
publication remains deferred.

Evidence:
the retired logs (Git history).

The private bridge proof packed `@inflatable-cookie/longhorn/core`, `@inflatable-cookie/longhorn-tauri`, and
`@inflatable-cookie/longhorn/bridge`, then installed a minimal consumer (retired 2026-10-22) and shapes for
Jetstream, Soundcheck, Nucleus and Loophole in isolated roots. Every consumer resolves version
`0.1.0`, one Tauri API `2.10.1` peer, exact declared bridge subpaths, and no
workspace or sibling source alias. Separate offline Rust consumers prove a
query-only graph without supervision/Tauri and a selected full-host graph.

The private Rust evidence uses `cargo package --list` plus exact source
archives. Registry-normalized Cargo packaging remains a release-lane gate.

Evidence:
the retired logs (Git history).

## Native-content Artifact Checkpoint

The private native-content proof inventoried five private Rust source artifacts after
`cargo package --list`, then compiled pure, child-view, isolated-window, and
backing-surface consumers offline on Rust 1.95. Registry-normalized Cargo
packages remain a release-lane gate.

It packs `@inflatable-cookie/longhorn/core`, `@inflatable-cookie/longhorn/native-content`, and
`@inflatable-cookie/longhorn-poodle-svelte/native-content` and installs isolated Nucleus, Soundcheck,
and Jetstream shapes. Each lock resolves version `0.1.0`, no workspace or
sibling source alias, only its selected mechanism, and one Svelte/Poodle
runtime where the Svelte adapter is selected. The exact five-package Poodle
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
the retired logs (Git history).

## Nucleus Private Migration Checkpoint

The Nucleus compatibility proof installed eight selected Longhorn TypeScript packages plus the exact
five-package Poodle g12.018 artifact set in a clean Nucleus-shaped renderer.
It compiles the public Poodle overlay geometry seam on Svelte 5.56.8,
TypeScript 6.0.3, and Tauri API 2.11.1. No Surface package resolves.

The Rust consumer compiles eleven selected Longhorn packages with Tauri
2.11.5 on Rust 1.95 from an exact offline lock. An unlocked Tauri 2.11
resolution can select transitive crates requiring newer Rust, so it does not
prove Longhorn's MSRV. The selector emits the evolved lock digest with the
artifact and source receipts.

The operator accepted both Nucleus layout checks after the dependency proof.
That evidence admitted the bounded storage cutover. It
makes no registry, publication, or public compatibility-range claim.

Evidence:
the retired logs (Git history).
