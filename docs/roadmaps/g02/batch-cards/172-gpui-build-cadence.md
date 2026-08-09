# 172 GPUI Build Cadence

Status: ready
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

## Do Not

- Move `gpui` into the default workspace. That is the trade this exists to
  avoid, and it would slow every unrelated Rust selector.
- Choose "hand-run" silently. It is a legitimate answer and an illegitimate
  default.

## Acceptance Criteria

- the cadence is chosen, written down, and reachable by name
- a change that breaks the prototype's compilation is caught by something
  other than a person remembering
- the papercut entry is closed or updated to say what was decided

## Evidence Required

- the measured build cost, cold and warm
- the selector, gate, or README line, whichever was chosen
- one deliberate break proving the mechanism catches it

## Stop Conditions

- the measured cost turns out to be small enough that the original exclusion
  was wrong, in which case the answer is to bring the prototype into the
  workspace and this card becomes that instead
