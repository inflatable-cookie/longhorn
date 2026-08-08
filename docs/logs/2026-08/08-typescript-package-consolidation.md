# TypeScript Package Consolidation

Date: 2026-08-08
Card: 164
Roadmap: g02.013

## Result

Eighteen published TypeScript packages are three, grouped by peer requirement
rather than by domain. 61 entry points became 62 — the only additions are the
two new package roots — and nothing that resolved before stopped resolving.

```
@inflatable-cookie/longhorn                 32 entries, no peers
@inflatable-cookie/longhorn-poodle-svelte   23 entries
@inflatable-cookie/longhorn-tauri           10 entries
```

## What The First Attempt Got Wrong

An earlier attempt was made and reverted. It treated the peered subpaths —
`config/poodle`, `history/svelte` — as leaf entry points that could be lifted
out of their domains. They are subsystems that reach back into domain
internals, and each is a directory as well as a file. Moving the entry files
orphaned the directories; moving the directories too took the type-error
count from 150 to 188.

The card was rewritten around one instruction: map the coupling before moving
anything.

## What The Map Showed

Across all 17 peered surfaces the reach into domain internals is 12 distinct
modules — `controller.ts`, `ports.ts`, `projectors.ts`, `keyboard.ts`,
`client.ts`, `registry.ts`, `connection.ts`, `session.ts`, `operation.ts`,
`index.ts`, `svelte.ts`, `generated/protocol.ts`.

Every one is already re-exported by its domain's `index.ts`. The peered
surfaces were never reaching private internals; they were reaching public
modules by relative path, because inside one package that costs nothing.

So nothing had to move with a surface and nothing had to be promoted to a new
export. The card's own revised estimate — that the export decisions would be
the real work, and would touch `longhorn`'s public surface under contract 012
— was wrong, and pessimistic. Measuring the coupling took one query and
turned a design problem back into a mechanical one.

## How The Move Was Driven

Not by pattern matching on specifiers. A deterministic old-to-new path map
was computed from the move rules, checked bijective against disk — 353 files,
zero predicted-but-absent, zero on-disk-unmapped — and only then used to
rewrite imports. Each relative specifier was resolved against its *old*
location, its target looked up in the map, and re-emitted as a new relative
path or a package specifier depending on which package the target landed in.
Zero unresolved imports.

The same map then repointed tsconfig `paths`, fixture paths, `packageRoot`
URLs, and the bindings generator's output constants.

## Two Latent Defects Fell Out

Consolidating the tsconfigs widened coverage, and two things had been hiding
under the old per-package scopes.

`packages/config/tests-svelte` was **never type-checked** — its tsconfig
included `src` and `tests` but not `tests-svelte`, while every other domain
with that directory included it. Two real cast errors were sitting in it.

`check:svelte` covered **873 files and none of Longhorn's own components**,
because it ran against a tsconfig whose include listed only `.ts`. The
consolidated config covers 2,995 and immediately found `LayoutSplitView`
importing `SplitToggleVisibility` from Poodle — a type Poodle 0.1.0 defines
but does not export from its package root. Contract 012 forbids reaching past
that root, so it is now derived via `ComponentProps<typeof SplitView>`, which
needs no upstream change.

Both are coverage gains that the split had been hiding, not regressions this
card introduced.

## Boundary Tests Got Stronger

The eighteen per-package `package.test.ts` files each asserted their own
manifest and could not survive as written. Replacing them with one
`tests/boundary.test.ts` per package tightened what they check: where each
old test listed the specific source files that must carry no host or UI edge,
the new one walks the whole `src` tree. The per-domain root symbol
assertions are preserved as a table.

## Pre-existing Failures, Measured Not Assumed

A `HEAD` worktree was built and run before landing, because two failures
looked merge-caused and were not:

- 11 of the 12 artifact proofs already failed at `HEAD`, on the stale
  `../poodle/.artifacts/g12.016-A698XB` pin. Only `bridge-topology-conformance`
  passed; it still does.
- `tests/poodle/split.test.ts` already failed at `HEAD`. Card 161 mapped
  region-hidden to SplitView `hidden`; the pinned Poodle pack still renders
  `data-primary-collapsed`.

Both are blocked on the Poodle release that already blocks the v0.1.0 tag.

## Remaining

Consumer migration. Nucleus, loophole, soundcheck and jetstream still name the
old eighteen. The frozen migration fixtures record that state deliberately, so
neither they nor the verifiers asserting against them were rewritten.
