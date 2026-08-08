# 164 TypeScript Package Consolidation

Status: complete — landed 2026-08-08 (first attempt reverted; see Attempt Notes)
Owner: Tom
Roadmap: g02.013 batch 1
Governing refs: contract 012; contracts 013 and 020
Depends on: none
Auto-start next card: no

## Objective

Collapse the eighteen published TypeScript packages into three, grouped by
peer requirement rather than by domain, before publication freezes the
names.

## Why — measured 2026-08-08

### The split buys nothing on the TypeScript side

Every domain package depends on `core` and nothing else. `layout` and
`commands` depend on nothing at all. No third-party dependency differs
between any of them.

Contract 012 states that "disabling Surfaces, history, commands, settings,
or service topology removes their dependency graph". On the **Rust** side
that is true and valuable — finch compiles 8 crates of 41, so the split
saves real work. On the **TypeScript** side it is false: removing `history`
removes `history`, because there is no tree beneath it to prune, and
tree-shaking already keeps unused exports out of an application bundle.

### Peer requirement is the only real axis

| peers | packages |
| --- | --- |
| none | core, layout, surfaces, transfer, surface-transfer, native-content |
| `poodle-svelte` + `svelte` | commands, config, history, history-tree, notifications, settings |
| `svelte` (+ longhorn) | svelte, native-content-svelte, operation, poodle |
| `@tauri-apps/api` | tauri |
| `longhorn-tauri` | bridge |

Those poodle and svelte peers on domain packages were already declared
**optional**, gating subpaths like `config/poodle` rather than the root
entry — contract 012 requires exactly that. So the mechanism that carries a
consolidated package already existed and was already in use.

### Svelte and Poodle are never used apart

| consumer | `longhorn-svelte` | `longhorn-poodle` |
| --- | --- | --- |
| nucleus | yes | yes |
| loophole | yes | yes |
| soundcheck | no | no |
| jetstream | no | no |
| figmatic | no | no |

Two use both, three use neither, **none uses one without the other**. And
`longhorn-poodle` already depended on `longhorn-svelte`, so installing poodle
had always pulled svelte. The split was never exercised.

### Version skew is a real failure mode the split created

Contract 012 mandates lockstep versioning across the repository, so
per-package versioning bought nothing. But eighteen lockstep packages
installed *independently* can skew — a consumer resolving
`longhorn-core@0.1.0` beside `longhorn-layout@0.1.1` was a live possibility
that one package makes impossible.

`longhorn-svelte` alone peered on four separate longhorn packages. In the
consolidated shape that is one peer, and four skew opportunities became
zero.

## Landed Shape

```
@inflatable-cookie/longhorn                 32 entries, no peers
@inflatable-cookie/longhorn-poodle-svelte   23 entries
    peers: svelte, @inflatable-cookie/poodle-svelte (optional), longhorn
@inflatable-cookie/longhorn-tauri           10 entries
    peers: @tauri-apps/api, longhorn
```

62 real entry points against 61 before — the only additions are the two new
package roots. Every entry that resolved before resolves now.

`longhorn-poodle-svelte` rather than `longhorn-poodle`: Poodle has a GPUI
implementation, and the Rust sibling will be `longhorn-poodle-gpui`. Naming
the framework now avoids renaming after publication.

## Attempt Notes — 2026-08-08, first attempt reverted

The first attempt moved 363 files and rewrote 344, then stalled and was
reverted cleanly. It assumed the peered subpaths — `config/poodle`,
`history/svelte` — were leaf entry points that could be lifted out. They are
subsystems that reach back into their domain's internals, and each is a
directory as well as a file. Moving the entry files orphaned the
directories; moving the directories too raised the type-error count from 150
to 188.

The revised instruction was to **map the coupling before moving anything**.

## What The Coupling Map Showed

Across all 17 peered surfaces, the reach into domain internals is **12
distinct modules**:

```
controller.ts   ports.ts     projectors.ts   keyboard.ts
client.ts       registry.ts  connection.ts   session.ts
operation.ts    index.ts     svelte.ts       generated/protocol.ts
```

**Every one of them is already re-exported by its domain's `index.ts`.** The
peered surfaces were never reaching into private internals; they were
reaching public modules by relative path because, inside one package, that
cost nothing. So no internal had to move with its surface, and no internal
had to be promoted to a new export. The card's own "revised estimate" — that
the export decisions would be the real work and would touch `longhorn`'s
public surface — was wrong, and pessimistic.

Once that was known the move was mechanical, and it was driven by a
deterministic old-to-new path map rather than by pattern matching: 353 files,
verified bijective against disk before a single import was rewritten, then
every relative specifier resolved against its *old* location and re-emitted
as either a new relative path or a package specifier depending on where its
target landed. Zero unresolved imports.

## Two Latent Defects Surfaced

Consolidating the tsconfigs widened type coverage, and two things had been
hiding under the old per-package scopes.

**`packages/config/tests-svelte` was never type-checked.** Its tsconfig
included `src` and `tests` but not `tests-svelte`; every other domain with a
`tests-svelte` directory included it. Two real cast errors were sitting
there. Fixed.

**`check:svelte` covered 873 files and none of Longhorn's own components.**
It ran against `packages/svelte/tsconfig.json`, whose include listed only
`.ts`. The consolidated config covers 2,995 files, and immediately found
`LayoutSplitView.svelte` importing `SplitToggleVisibility` from
`@inflatable-cookie/poodle-svelte` — a type Poodle 0.1.0 defines but does not
re-export from its package root. Since contract 012 forbids reaching past
that root, it is now derived from the public component via
`ComponentProps<typeof SplitView>`, which needs no upstream change.

## Boundary Tests

The eighteen per-package `package.test.ts` files each asserted their own
manifest, so none could survive the merge as written. They were replaced by
one `tests/boundary.test.ts` per package, and the separation invariants they
encoded got **stronger** rather than being dropped: where each old test
listed the specific source files that must stay free of a host or UI edge,
the new one walks the entire `src` tree. The per-domain root symbol
assertions — `NotificationClient` present, `createTauriNotificationPort`
absent — are preserved as a table.

## Evidence

- entry-point inventory before and after: 61 → 62, nothing dropped
- `fmt:rust`, `test:rust`, `check:ts`, `check:svelte`, `check:packages`,
  `test:ts`, `test:vitest`, `check:bindings`, `check:api-reference`,
  `held-surface`, `host-protocol`, and every docs gate pass
- `check:bindings` regenerates with no semantic diff after the generator was
  repointed at the consolidated layout
- `docs/reference/api-surface.md` regenerated; `package-topology.md` and
  contract 012 refreshed

## Known Failures, All Pre-existing

Measured against a `HEAD` worktree before landing, not assumed:

- **11 of the 12 artifact proofs already failed at `HEAD`**, on the stale
  `../poodle/.artifacts/g12.016-A698XB` pin recorded in PAPERCUTS. Only
  `bridge-topology-conformance` passed, and it still passes.
- **`tests/poodle/split.test.ts` already failed at `HEAD`.** Card 161 mapped
  region-hidden to SplitView `hidden`, and the pinned Poodle pack still
  renders `data-primary-collapsed`. Blocked on the same Poodle release that
  blocks the tag.

## Remaining

**Card 149's candidate receipt needs regenerating.** It freezes package
counts, and those counts changed. Its scripts' `startsWith("…/longhorn-")`
predicates were widened here so they see the consolidated root, but the
receipt itself stays operator-held on consumer manifest quiescence and cannot
be produced yet. Card 127's superseded scripts were left alone.

The frozen migration fixtures under `fixtures/migration/` and the verifiers
asserting against them still record the old eighteen names, deliberately —
they are evidence of what consumers looked like at those cards' dates, not
live inventory.

## Consumer Migration

Five consumers, not the four this card listed: **figmatic** also imports
Longhorn, through vite aliases rather than manifest dependencies, which is
why the package inventory missed it.

| consumer | validated by | result |
| --- | --- | --- |
| nucleus | `svelte-check`, 23 tests, `check:longhorn-consumer` | green |
| loophole (aura) | renderer build | green |
| soundcheck | `svelte-check` + build, 56 tests | green |
| jetstream (editor-ui) | 67 tests, build | green |
| figmatic (studio) | build | green |

Three things the migration exposed that the card had not anticipated:

**Domain `/tauri` subpaths became a new dependency.** A consumer using
`longhorn-notifications/tauri` got the host edge inside the domain package it
already had. Now those edges live in `longhorn-tauri`, so nucleus and
soundcheck needed it added explicitly.

**Peer satisfaction needs an override under `file:` refs.** The two peered
packages declare `@inflatable-cookie/longhorn` at `0.1.0`, which a bare
`file:` dependency does not satisfy on its own. Nucleus and soundcheck
already carried overrides and were fine; jetstream did not, and 404'd against
the registry until one was added.

**Nucleus's boundary claim had to weaken, honestly.** It asserted that
Surfaces, history and surface-transfer were not installed. With one package
that is no longer true or expressible — those domains ship whether composed
or not, and tree-shaking is what keeps them out of a bundle. The check moved
to source absence, which is the half that ever mattered, and the packed
artifact proof gained a stronger assertion in its place: exactly three
Longhorn packages resolve and no fourth leaked in. Rust crate selection is
untouched, since that split is still real.

Two pre-existing defects surfaced and were fixed in passing. Figmatic's vite
aliases still named the retired `@longhorn/*` scope, so nothing they listed
matched what the studio imports — its Longhorn resolution had been broken
since that rename. And soundcheck's committed lockfile recorded Poodle's
pre-rename peers at `0.0.0`, while current Poodle source wants
`0.1.0` of four siblings that exist on no registry; those are now overridden
to Poodle's source directories.

## Acceptance Criteria

- [x] three published packages; every current entry point still resolves
- [x] optional peers still gate the subpaths that need them
- [x] no consumer can install a skewed pair of longhorn packages
- [x] bindings regenerate with no semantic diff
- [x] all consumers compile and their suites pass (five, not four)
- [x] contract 012 no longer claims a dependency-graph benefit the TypeScript
      side does not deliver

## Timing

**Corrected 2026-08-08.** This card said "before poodle publishes", which
was wrong: poodle publishing does not freeze *longhorn's* names. Longhorn's
own publication does, and that is still several steps out — poodle
publishes, longhorn moves off `file:` refs, CI runs its TypeScript lane,
then a tag. There was more room than the original framing claimed.

## Notes

Weigh against the fact that this layer is webview-edge tier. Roughly 18,000
of its 24,000 source lines are IPC machinery — generated protocol,
validators, transport wiring — that a GPUI application never composes. This
was worth doing because publication freezes names, not because the layer is
growing.

This only became a live question when the operator chose public npm on
2026-08-08. Privately, nobody would have hit a mismatched-version install;
publicly, three packages is a materially better front door than eighteen.
