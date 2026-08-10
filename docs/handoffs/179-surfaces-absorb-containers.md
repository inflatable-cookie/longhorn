# Handoff — Execute Card 179, Surfaces Absorb Containers

Paste the section below into a fresh thread. Everything above the rule is
context for whoever is doing the pasting.

This exists because Card 179 has no compiling intermediate: it is one
uninterrupted pass across twelve crates, so it needs a thread with a full
context budget rather than the tail of a long session.

---

You are working in `~/Dev/projects/longhorn`, a Rust + TypeScript library for
composable desktop workspace hosting. Your task is to execute one card.

**Read `docs/roadmaps/g02/batch-cards/179-surfaces-absorb-containers.md` first
and in full.** It holds the target model, the crate decision, the migration, the
sweep order and the reasoning behind each. Do not re-litigate its decisions —
they were taken deliberately with the repository owner. This prompt covers only
what the card does not.

## What you are doing, in one sentence

`LayoutContainerId` leaves the model entirely: a Surface owns its own schema,
regions and sizing slots, `longhorn-layout` and `longhorn-layout-config` are
absorbed into `longhorn-surfaces` and `longhorn-surfaces-config`, and contract
014 folds into 002 as a superseded stub.

## Why it blocks a release

Poodle 0.1.0 published on 2026-08-10. Longhorn is next and is unpublished, with
all six of its consumers still pinning it by `file:`. `LayoutContainerId` on a
public wire is exactly the mistake the publication work exists to avoid, so this
lands before the tag. Nothing external pays for the delay.

## How to run anything

The repository uses Effigy, not package scripts. `effigy tasks` lists
selectors. The ones you need:

```sh
effigy check:ts          # TypeScript typecheck
effigy check:bindings    # generated protocol matches Rust
effigy generate:bindings # regenerate it
effigy proof:artifacts   # twelve consumer proofs, slow
effigy qa                # everything; several minutes
```

`cargo check --workspace` is your main driver during the sweep — the card's
order exists so the compiler enumerates the work for you.

## Things that will cost you an hour if nobody tells you

**The checkout is shared with other agents.** Stage by explicit path, always.
Never `git stash`, never `git reset --hard`, never `git add -A`. Another thread
has restored files staged for deletion mid-task before.

**There is no compiling intermediate.** The moment `LayoutDocument` stops
existing, the dependent crates stop compiling and stay that way until the sweep
finishes. Do not commit a broken tree hoping to fix it next commit. If you must
stop, revert cleanly — a previous attempt did exactly that and the revert is
recorded in the card.

**Do not run a blanket text substitution over prose and code.** Card 167 did,
and it broke a release gate by writing a placeholder into an executable script,
and broke two crates by renaming Rust identifiers that cannot contain a hyphen.
`container` also appears in documentation meaning the ordinary English word.

**`bun build` is not a syntax check.** It resolves imports and will fail on an
unrelated missing module while saying nothing about your file. Use
`new Bun.Transpiler({loader:"ts"}).transformSync(source)` if you need to check
that a file parses.

**Unmatched globs kill a zsh loop.** `for f in some/*.ts` aborts the whole
command when nothing matches, so a loop that reports zero for every input
probably never ran. Use `find` or Python.

**The bindings generator must declare every type it references.** Adding a Rust
type is not enough: add it to the declaration list in
`crates/longhorn-bindings/src/surfaces.rs`, or the generated `protocol.ts` will
reference a type it never defines. `check:bindings` will still pass, because it
compares generated output against committed output and both agree when the
generator is wrong. `check:ts` is what catches it. This is an open papercut.

**The golden fixture must cover every discriminant.** A TypeScript test asserts
that the fixture exercises every mutation command, outcome and rejection code.
Add new commands to `crates/longhorn-bindings/src/surfaces/fixture.rs`;
receipts are derived by running the engine, so outcomes follow automatically.

**Example consumers construct document literals.** At least
`examples/app-shell-proof/loophole/src/model.ts` builds a `SurfaceRecord` by
hand and will need every new required field.

## Verifying you are done

`effigy qa` green is the bar, including `check:bindings` and all twelve
artifact proofs.

One proof can fail for reasons that are not yours: `verify-greenfield-card125.ts`
packs Poodle's **live working tree** from `../poodle`, so if that tree has
in-progress work with a type error, greenfield goes red. Check
`git -C ../poodle status` before assuming you caused it.

## Related state

- **Card 177** landed on 2026-08-10 and added `SurfacePresentation` to
  `SurfaceRecord`. Its container invariant is currently a documented consumer
  obligation in contract 002; Card 179 turns it into a local engine check and
  deletes that stated ceiling.
- **Card 178** is superseded by 179 and should not be implemented.
- The Loophole consumer has been told its add-surface button resolves through
  Surface creation, not a `CreateContainer` command.

## Out of scope

Do not publish anything, do not tag, and do not touch the release workflows.
Publication is coordinated separately by the repository owner.
