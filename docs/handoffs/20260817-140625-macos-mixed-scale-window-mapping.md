---
title: macOS mixed-scale window mapping handoff
kind: northstar-handoff
status: complete
owner: Tom
created: 2026-08-17
updated: 2026-08-17
completed: 2026-08-17
outcome: >-
  Card 226 landed on main. MacOsDesktopMapper reads the platform's own logical
  plane; contract 009 and the architecture note carry the policy. Figmatic
  adoption is the explicit next task and was not performed from this lane.
  This file is history, not a dispatch.
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260817-140625-macos-mixed-scale-window-mapping.md
tags: [coordination, handoff, windowing, macos]
---

## What This Thread Was Doing

Figmatic was being reviewed on a Mac connected to an external display when its
hidden-window restore failed during startup:

```text
figmatic hidden restore: Figmatic hidden window restore failed on attempt 8: observe Figmatic desktop failed: Projection(Mapping(MixedScaleUnavailable { scales: [ScaleFactor(1000), ScaleFactor(2000)] }))
```

The Figmatic thread traced this to Longhorn's deliberate mixed-scale boundary.
Figmatic currently supplies `UniformScaleMapper` for both the initial desktop
observation and the post-apply readback. Longhorn rejects the complete snapshot
when the desktop contains both 1x and 2x displays.

## Why It Matters

Window size and position persistence is a baseline Longhorn integration for
Figmatic. A normal MacBook plus external-monitor arrangement currently prevents
the app from completing hidden restore, so the window never reaches its normal
startup path. Removing saved state cannot fix it because observation fails
before restore planning reads that state.

This is the production capability deliberately left behind Longhorn's injected
whole-desktop mapper seam. The consumer has now supplied the evidence needed to
characterize and close that gap.

## Current State

Here is the short version of where things stand:

- **Done:** The failure is reproduced by consumer evidence and localized to the
  mixed-scale coordinate-mapping boundary. Longhorn already has
  `DesktopCoordinateMapper`, complete physical desktop snapshots, typed refusal,
  and an injected-mapper test seam.
- **Still open:** Longhorn has no production macOS mapper that establishes one
  coherent logical desktop plane across mixed-scale displays and managed
  windows.
- **Active spec lane:** None yet. This is new consumer evidence against the
  completed g01 windowing lane and needs promotion through the current docs
  spine before execution.
- **Current batch card:** None. Card 017 is historical authority, not an active
  card to reopen silently.
- **Canonical refs:**
  `/Users/tom/Dev/projects/longhorn/docs/contracts/009-display-identity-coordinates-and-window-planning.md`
  and
  `/Users/tom/Dev/projects/longhorn/docs/architecture/tauri-window-host-integration.md`.
- **Remaining continuation envelope:** Characterize the macOS production mapper,
  promote any required architecture/contract clarification, compile a bounded
  card, then implement and prove it. Do not disturb the unrelated live g02
  pointer.
- **Lane budget / pause signal:** Pause if native macOS APIs cannot correlate
  physical and logical display/window geometry without adding a new identity or
  host-authority boundary.
- **Key files:**
  - `/Users/tom/Dev/projects/longhorn/crates/longhorn-tauri-windowing/src/mapping.rs`
  - `/Users/tom/Dev/projects/longhorn/crates/longhorn-tauri-windowing/tests/observation/mapping.rs`
  - `/Users/tom/Dev/projects/longhorn/docs/roadmaps/g01/batch-cards/017-tauri-display-and-live-window-observation.md`
  - `/Users/tom/Dev/projects/longhorn/docs/logs/2026-07/29-windowing-first-consumer-follow-up.md`
  - `/Users/tom/Dev/projects/figmatic/crates/figmatic-studio/src/window_host.rs`

## Boundaries

Please keep the next pass within these boundaries:

- **In scope:** A production macOS whole-desktop coordinate mapper for mixed 1x
  and 2x display arrangements, its public composition seam, tests, native proof,
  and precise Figmatic adoption instructions.
- **Out of scope:** Editing Figmatic from the Longhorn lane, changing product
  restore policy, fabricating geometry through per-monitor origin division, or
  weakening `UniformScaleMapper`'s fail-closed contract.
- **Repo constraints:** Follow
  `/Users/tom/Dev/projects/longhorn/AGENTS.md` and the canonical architecture and
  contracts above. Longhorn remains consumer-neutral and pre-1.0; do not add a
  compatibility shim or silent fallback.

## Important Context

- **Planning lineage:** g01 Card 017 intentionally introduced the injected
  whole-desktop mapper and rejected mixed-scale snapshots without one. The
  2026-07-29 first-consumer follow-up and the 2026-08-01 Nucleus migration both
  recorded mixed-scale production support as deferred. This is fresh Figmatic
  consumer evidence, not a regression in the uniform mapper.
- **How the plan fits the system:** Tauri supplies physical monitor and window
  facts. Longhorn's contract requires a platform mapper to establish one
  coherent logical coordinate plane for the complete snapshot. The solution
  belongs at the macOS adapter edge and should remain injectable/testable behind
  `DesktopCoordinateMapper`.
- **Decisions and preferences:** Keep typed failure when no capable platform
  mapper is available. Use authoritative native macOS coordinate evidence; do
  not divide every monitor origin by its own scale and call the result global
  DIPs. Initial observation and apply readback must use the same mapping policy.
- **Open tensions:** The exact native API and display-correlation method have not
  been selected. Confirm how AppKit/Core Graphics coordinate systems, origin
  orientation, menu-bar placement, work areas, negative origins, and managed
  window frames compose before freezing the API. Also decide whether this is a
  macOS-only mapper exposed directly or a platform mapper selected by a shared
  production constructor.

## Suggested Next Move

Start by checking the current contract, architecture note, Card 017, its tests,
and the two consumer follow-up logs named above. Then characterize the native
macOS mapping evidence and propose the smallest production API that can map the
complete `PhysicalDesktopSnapshot` without weakening existing failure
semantics.

The proof should cover at least:

- mixed 1x/2x displays in left, right, above, and negative-origin arrangements;
- input-order invariance and windows resident on each display;
- full bounds, work areas, outer bounds, and inner sizes;
- initial observation and fresh post-apply readback using the same mapper;
- restore/readback convergence; and
- a packaged macOS run on a genuinely mixed-scale desktop.

When the Longhorn side is complete, return the exported API and the exact two
Figmatic substitutions required in
`/Users/tom/Dev/projects/figmatic/crates/figmatic-studio/src/window_host.rs`.
Figmatic currently passes `UniformScaleMapper` to `observe_tauri_desktop` and
to `TauriDesktopReadback::new`.

## Completion Protocol

This handoff exists because the Longhorn thread needs to own the shared-system
decision and delivery. Before finishing:

1. Promote the chosen coordinate policy into the appropriate architecture and
   contract surfaces before implementation outruns them.
2. Compile and close a bounded roadmap card without replacing Longhorn's
   unrelated live generation pointer.
3. Add unit, projection, and native packaged evidence proportional to the new
   platform boundary.
4. Run focused checks while developing, then `effigy qa` for the completed
   batch. Respect the Rust release floor; `effigy ci:rehearse` is needed only if
   the work is taken through a release.
5. Record the completed batch in the roadmap and logs, including any unexecuted
   display arrangements.
6. Return a reviewable PR from a dedicated non-`main` worktree. Do not merge it
   from the worker thread.
7. Leave Figmatic adoption as the explicit next task, with exact API usage and
   any minimum Longhorn revision stated.

The unresolved risk is native coordinate correlation. If the available macOS
evidence cannot establish a coherent plane for every observed display and
managed window, stop at the typed boundary and bring the missing contract
choice back to the operator.
