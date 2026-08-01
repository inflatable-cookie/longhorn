# Nucleus Protected Window Host Cutover

Date: 2026-08-01
Card: 097
Outcome: `pass`

## What Changed

- Replaced Nucleus `window_geometry.rs` with the protected predeclared
  Longhorn window host.
- Kept logical `window:primary` separate from Tauri label `main`.
- Added canonical display registry, saved placement, fallback, clamping,
  maximized restore, and explicit 2x physical/logical mapping.
- Added hidden apply plus two-signal reveal through renderer page readiness.
- Added settled lifecycle capture, programmatic attribution, blur persistence,
  one-second close flush, and aggregate exit teardown.
- Registered machine-state domain `nucleus.window-placement` through
  `ConfigWindowPlacementSink`.
- Converted Card 096 raw placement state through verified backup and a
  completion receipt. Interrupted receipt publication resumes without replay.
- Removed renderer placement write authority and unconditional native reveal.
- Fixed the shared Tauri probe for a valid primary monitor paired with an
  empty available-monitor list. Mutable work area is not a primary identity
  field.

## Evidence

Nucleus commit `fa7f06e7dfdf4a8bde7f4ab48df360a3087a05e7` consumes
Longhorn commit `32f4bd66e7df813af07514c654ce3b03ddc21ccd`. The prior
Nucleus authority point is `1c0f8e225849718888afdba63cee4745f623e906`.
The Nucleus lockfile SHA-256 is
`ca01828707f2f428af5cd348d5e6f3bbd6d61df67f8854a63cd5a366cf615f17`.

The isolated macOS arm64 native run used `portable-v1`. Fresh start and restart
both converged on hidden apply attempt one. The run observed a 2x 3600x2338
physical display, mapped it to 1800x1169 logical units, and persisted
`nucleus-display:0` with normal logical placement 1280x820 at 260,117. Restart
exposed Tauri's valid-primary/empty-available edge; the shared fix admits the
validated primary as the sole observation only when the available set is
empty. Non-empty inconsistent or ambiguous sets still fail.

Validation passed:

- 71 Nucleus desktop Rust tests
- 40 Nucleus renderer tests
- Nucleus Rust workspace, Svelte, and Northstar checks
- all 44 `longhorn-tauri-windowing` tests
- strict Longhorn windowing Clippy
- fresh and restart native launch
- unchanged main-only capability; no wildcard or remote child authority
- no Surface crate in the Nucleus desktop graph

The Svelte check retains one pre-existing ProjectRail ARIA warning. Strict
whole-package Nucleus Clippy remains blocked by unrelated existing warnings in
storage migration, command policy, agent protocol, and desktop constructors.
The new host compiles without warnings. The user-owned
`TerminalPanel.svelte` edit remained unstaged.

## Decisions

The existing legacy display string is not canonical evidence. Migration keeps
its geometry and maximized state with an unresolved display association; fresh
observation assigns canonical identity on settled capture.

Mixed-scale desktops remain fail-closed under `UniformScaleMapper`. Nucleus
does not fabricate a global logical plane. A later platform mapper can expand
support without changing the placement domain.

Package-manager publication remains deferred. Card 098 can transfer the
project-keyed five-region layout while this global window domain stays
independent.
