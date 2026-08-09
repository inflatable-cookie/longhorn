# 173 GPUI Composition Guide

Status: ready
Owner: Tom
Roadmap: g02.015
Governing refs: contract 020; contract 013; memo 022
Depends on: Card 169; Card 171
Auto-start next card: no

## Objective

One document that gets a GPUI application from nothing to a window with
something in it.

## Why this exists

Every piece exists and nothing says how they fit. `longhorn-gpui-windowing`
opens and places windows, `longhorn-poodle` projects six domains,
`poodle-render` and `poodle-gpui-node-backend` draw them, and the only place
the assembly is written down is a prototype binary nobody is meant to copy.

Memo 022's D8 is the sharpest reason. A GPUI application must supply request
ids, date formatting and locale — three things a webview gives away free — and
nothing currently tells an author that. `HostServices` bundles them, and a
bundle nobody knows to implement is not better than three parameters.

## Scope

- the guide, in `docs/guides/`
- the assembly: host adapter, lifecycle host, projection tier, `HostServices`
- what a GPUI application owns that a Tauri one does not
- what it does **not** need: the whole webview command surface, which is most
  of what `longhorn-tauri-*` weighs

## Steps

1. Write the assembly in order — window backend, lifecycle host, services,
   projection — each with the minimum a caller must decide.
2. Name every seam. `HostServices`, `GpuiWindowBackend`,
   `GpuiWindowCaptureBackend`, `GpuiLifecycleScheduler`,
   `GpuiUserCloseHandler`, `WindowPlacementSink`. A seam absent from this list
   is a seam an author discovers from a compiler error.
3. State the difference from Tauri honestly, including where GPUI is worse:
   no move, no show, no hide; display facts need a per-platform reader.
   Contract 020 already records these and the guide should not soften them.
4. Cross-reference rather than restate. The contract is the authority; this is
   the path through it.

## Do Not

- Invent API. If the assembly cannot be written without a surface Longhorn does
  not have, that is a contract 020 gap and the milestone's planning checkpoint
  fires.
- Duplicate the contract. A guide that restates requirements drifts from them.

## Acceptance Criteria

- an author who has not read the adapter source can assemble a window
- every seam a GPUI application must implement appears in one list
- the guide names at least one thing GPUI does worse than Tauri

## Evidence Required

- the guide, with its example compiling under Card 174
- a link from `docs/guides/getting-started.md` or the guides front door

## Stop Conditions

- the assembly needs surface that does not exist, in which case stop and
  raise it against contract 020 rather than writing aspirational API
