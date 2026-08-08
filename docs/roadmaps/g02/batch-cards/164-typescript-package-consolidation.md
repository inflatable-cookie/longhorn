# 164 TypeScript Package Consolidation

Status: ready — first attempt reverted 2026-08-08; see Attempt Notes
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

Those poodle and svelte peers on domain packages are already declared
**optional**, gating subpaths like `config/poodle` rather than the root
entry — contract 012 requires exactly that. So the mechanism that would
carry a consolidated package already exists and is already in use.

### Svelte and Poodle are never used apart

| consumer | `longhorn-svelte` | `longhorn-poodle` |
| --- | --- | --- |
| nucleus | yes | yes |
| loophole | yes | yes |
| soundcheck | no | no |
| jetstream | no | no |
| figmatic | no | no |

Two use both, three use neither, **none uses one without the other**. And
`longhorn-poodle` already depends on `longhorn-svelte`, so installing poodle
has always pulled svelte. The split has never been exercised.

### Version skew is a real failure mode the split creates

Contract 012 mandates lockstep versioning across the repository, so
per-package versioning buys nothing. But eighteen lockstep packages
installed *independently* can skew — a consumer resolving
`longhorn-core@0.1.0` beside `longhorn-layout@0.1.1` is a live possibility
that one package makes impossible.

`longhorn-svelte` alone peers on four separate longhorn packages. In the
consolidated shape that is one peer, and four skew opportunities become
zero.

## Target

```
@inflatable-cookie/longhorn
    core, config, layout, surfaces, transfer, surface-transfer, history,
    history-tree, commands, settings, notifications, operation,
    native-content, bridge
    peers: none

@inflatable-cookie/longhorn-poodle-svelte
    reactive bindings and Poodle projections
    peers: svelte, poodle-svelte, longhorn

@inflatable-cookie/longhorn-tauri
    invoke/listen transport
    peers: @tauri-apps/api, longhorn
```

`longhorn-poodle-svelte` rather than `longhorn-poodle`: Poodle has a GPUI
implementation, and the Rust sibling will be `longhorn-poodle-gpui`. Naming
the framework now avoids renaming after publication.

## Scope

- eighteen packages become three
- subpath exports preserve every current entry point
- generated bindings emit into the consolidated layout
- consumer migration across nucleus, loophole, soundcheck, jetstream

## Steps

1. Amend contract 012 first. Its dependency-graph claim is true for Rust and
   false for TypeScript, and this card is the evidence. State the tiers
   separately rather than letting one sentence cover both.
2. Merge the fourteen peerless packages into `@inflatable-cookie/longhorn`,
   preserving every public entry point as a subpath. Nothing that resolves
   today may stop resolving.
3. Merge `svelte`, `poodle` and `native-content-svelte` into
   `longhorn-poodle-svelte`.
4. Repoint the bindings generator at the consolidated layout and regenerate.
   `check:bindings` must stay green.
5. Migrate the four consumers. They import
   `@inflatable-cookie/longhorn-<domain>` today; those become subpaths.
6. Refresh the package-count claims: `docs/architecture/package-topology.md`,
   `docs/reference/api-surface.md`, and Card 149's candidate receipt, which
   freezes counts.

## Attempt Notes — 2026-08-08, reverted

A full attempt was made and reverted cleanly. Nothing was committed; the
repository is back at 18 packages with every gate green. What it learned
changes the plan, so a second attempt should start from here rather than
from the steps above.

### What worked

The bulk is genuinely mechanical and went in without trouble:

- 363 source and test files moved into three packages with `git mv`
- 344 files had their import specifiers rewritten by a literal map, using a
  sentinel pass so `longhorn-poodle-svelte` was not caught by the
  `longhorn-poodle` rule and `longhorn-tauri` survived intact
- 60 entry points expressed across three exports maps: 32 on `longhorn`,
  19 on `longhorn-poodle-svelte`, 10 on `longhorn-tauri`

### What stopped it

**The peered subpaths are not leaf entry points.** The card assumed
`config/poodle`, `history/svelte` and friends were single files that could
be lifted out. They are subsystems that reach *back* into their domain's
internals:

    packages/commands/src/poodle.ts     imports ./projectors.ts
    packages/commands/src/poodle/…      imports ../controller.ts
    packages/commands/src/svelte.ts     imports ./svelte/session.svelte.ts

Inside one package those are free. Split across packages, every one becomes
a cross-package import of something `longhorn` does not export.

**Each peered surface is a directory as well as a file.** Six domains carry
`src/<domain>/svelte/` and seven carry `src/<domain>/poodle/`, alongside the
`svelte.ts` and `poodle.ts` entry files. Moving only the entry files leaves
the directories orphaned; moving the directories too raised the type-error
count from 150 to 188, because their own relative imports then pointed
across the new boundary in the opposite direction.

**Test files break on depth.** Tests moved from
`packages/<domain>/tests/` to `packages/longhorn/tests/<domain>/`, so every
`../src/x` needs re-rooting. Mechanical, but it interacts with the above.

### What a second attempt should do first

Map the coupling before moving anything. For each of the 13 peered surfaces,
list which domain internals it imports. That set is the real decision:

- internals only the peered surface uses **move with it**
- internals shared with the domain's public client **become exports of
  `longhorn`**, which is a public-surface decision, not a mechanical one
- anything reaching in both directions means the boundary is wrong and the
  domain needs splitting differently

Only once that map exists is the move mechanical. Attempting it in the other
order produces a half-migrated graph, which is worse than either end — the
reason this attempt was reverted rather than pushed through.

### Revised estimate

Larger than the card implied. The file movement is an afternoon; the
coupling map and the export decisions it forces are the actual work, and
they touch `longhorn`'s public surface, which contract 012 governs.

## Acceptance Criteria

- three published packages; every current entry point still resolves
- optional peers still gate the subpaths that need them
- no consumer can install a skewed pair of longhorn packages
- bindings regenerate with no semantic diff
- all four consumers compile and their suites pass
- contract 012 no longer claims a dependency-graph benefit the TypeScript
  side does not deliver

## Evidence Required

- an entry-point inventory, before and after, proving nothing was dropped
- the four consumer migrations
- refreshed counts wherever the old numbers are asserted

## Stop Conditions

- an entry point cannot be preserved as a subpath without re-exporting an
  optional peer from a root entry, which contract 012 forbids

## Timing

**Corrected 2026-08-08.** This card said "before poodle publishes", which
was wrong: poodle publishing does not freeze *longhorn's* names. Longhorn's
own publication does, and that is still several steps out — poodle
publishes, longhorn moves off `file:` refs, CI runs its TypeScript lane,
then a tag. There is more room than the original framing claimed.

**Before longhorn publishes.** Published names freeze, and this changes
longhorn's names. It is also cheap now and expensive later: eighteen
published packages cannot be collapsed without deprecating seventeen of
them.

## Notes

Weigh against the fact that this layer is webview-edge tier. Roughly 18,000
of its 24,000 source lines are IPC machinery — generated protocol,
validators, transport wiring — that a GPUI application never composes. This
is worth doing because publication freezes names, not because the layer is
growing.

This only became a live question when the operator chose public npm on
2026-08-08. Privately, nobody would have hit a mismatched-version install;
publicly, three packages is a materially better front door than eighteen.
