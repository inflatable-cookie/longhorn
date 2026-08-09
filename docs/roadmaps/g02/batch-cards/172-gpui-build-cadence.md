# 172 GPUI Build Cadence

Status: complete
Completed: 2026-08-09
Owner: Tom
Roadmap: g02.015
Governing refs: contract 020; contract 001
Depends on: none
Auto-start next card: no

## Objective

Decide where a `gpui` build runs, and make that decision real.

## Why this exists

`prototypes/gpui-windowing` is the only artefact proving Longhorn's host seam
matches the real SDK, and it is the one thing CI never builds. It sits outside
the workspace because `gpui` adds several hundred transitive crates and a Metal
shader build to `lint:rust`, `lint:rust:features`, `test:rust` and `docs:rust`,
and taxing every run for one prototype was the wrong trade.

The consequence has been paid three times already this generation: the smoke
binary, the multiwindow binary and the render binary each found something no
in-memory test could, and each was verified by a person remembering to run it.
The render binary was broken by a signature change in the same session that
introduced it, and only a hand-run caught it.

Recorded in `PAPERCUTS.md` as "Heavyweight host SDKs have no in-gate home".

Siting an example before deciding where it builds is exactly how `prototypes/`
became ungated, so this card comes before the example that would join it.

## Scope

- the cadence decision, written down with its reason
- whichever mechanism the decision implies, working
- nothing about what the prototypes *contain*

## Steps

1. Measure the real cost. Time a cold and warm `cargo build` of
   `prototypes/gpui-windowing` and record both. The decision should be made
   against a number, not against "it is heavy".
2. Choose one:
   - **a nightly or scheduled Effigy selector** that builds excluded prototypes
     off the `qa` path
   - **a release gate**, so prototypes are proved before a tag and not before a
     commit
   - **explicitly hand-run**, documented as such, with the risk accepted in
     writing rather than by omission
3. Implement it. A selector needs an `effigy.toml` entry and a workflow step;
   a release gate needs the release workflow; "hand-run" needs a line in the
   prototype README saying so and why.
4. If the choice is a workflow change, get approval before editing anything
   under `.github/workflows/`.

## Result

### The measurement

| | |
| --- | --- |
| packages in the graph | 757 |
| cold build, wall | 37s |
| cold build, CPU | 349s |
| warm build, wall | 5.6s |
| linked artifacts | 3.3 GiB (6.2 GiB before `cargo clean`) |
| `check:prototypes`, warm, all six | 1.3s |

The impression that led to the exclusion was "several hundred crates and a
Metal shader build". Both true. The conclusion drawn from it — that this is
expensive — is true of disk and CPU and not of wall clock: 37s cold on this
machine, because 349s of CPU parallelises.

### The decision: a named selector, wired to release

Not the workspace. Adding `gpui` there taxes `lint:rust`,
`lint:rust:features`, `test:rust` and `docs:rust`, doubles under
`--all-features`, and puts 3.3 GiB in every cache — for one prototype, on a
CI runner with a fraction of this machine's cores.

Not nightly either. This repo's CI deliberately does not run on pushes; it
runs on pull requests and tags, because it exists to prove a clean clone. A
nightly would be the only trigger in the file that fires on nothing in
particular.

So: `check:prototypes`, `cargo check --all-targets --locked` across all six
prototypes, outside `qa`, in `release:gates` and `[release.gates]`. `check`
rather than `build` because the failure mode is API drift, which type checking
catches, and linking is where the cost is.

All six prototypes check clean today, which is worth recording — the selector
starts green rather than inheriting a backlog.

### It bites

Renaming `project_notification_stack` to take a second argument failed the
selector with `E0061` on the render binary. Restored and re-verified green. The
first attempt at this proof broke a symbol no prototype used and the selector
stayed green, correctly — which is its own small piece of evidence.

## Do Not

- Move `gpui` into the default workspace. That is the trade this exists to
  avoid, and it would slow every unrelated Rust selector.
- Choose "hand-run" silently. It is a legitimate answer and an illegitimate
  default.

## Acceptance Criteria

- [x] the cadence is chosen, written down, and reachable by name —
  `effigy check:prototypes`
- [x] a change that breaks the prototype's compilation is caught by something
  other than a person remembering
- [x] the papercut entry is closed and says what was decided

## Evidence Required

- the measured build cost, cold and warm
- the selector, gate, or README line, whichever was chosen
- one deliberate break proving the mechanism catches it

## Stop Conditions

Did not fire, but it was close enough to be worth saying why. 37s cold is not
obviously too expensive, and the stop condition would have triggered on wall
clock alone. What kept the exclusion is the shape of the cost rather than its
size: it lands on four selectors, doubles under `--all-features`, and 3.3 GiB
goes into every cache — all of it paid by every unrelated Rust change.
