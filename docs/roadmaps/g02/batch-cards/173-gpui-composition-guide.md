# 173 GPUI Composition Guide

Status: complete
Completed: 2026-08-09
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

## Result

`docs/guides/gpui-composition.md`, linked from the guides index, from
`getting-started.md`, and from `system-composition.md` — which now says in its
first paragraph that it is the Tauri path, because it never did.

### The guide leads with what the host does not give you

The seams table comes second and the platform table comes first: request ids,
dates, case folding. That ordering is the finding from memo 022's D8 made
structural. A GPUI author's first surprise is not that `PlatformWindow` lacks
`move` — it is that nothing formats a date.

Eight seams are named in one list. Every one existed before this card; none
was written down together.

### Nothing was invented

The stop condition was that the assembly might need surface Longhorn does not
have. It did not fire. Every symbol in the guide is checked against
`lib.rs` exports and real signatures, and the one place the guide could have
softened a limitation — `WITHHELD_CAPABILITIES`, display facts, the close
decision — it states them as contract 020 does.

Two things the guide says that no other document does:

- **`poodle-gpui` is the older tier.** Its `RenderComponent::render` returns a
  two-field handle and draws nothing; `poodle-render` is the component tier and
  `poodle-gpui-node-backend` interprets it. I had this backwards for most of a
  day and the guide is where the correction becomes load-bearing.
- **`project_notification_stack`, not `project_notifications`.** `Toast` is a
  leaf; the stack renders. Easy to get wrong from the function names alone.

### What it deliberately does not do

No API is restated. The contract is the authority and a guide that duplicates
requirements drifts from them; this is the path through, with links.

## Do Not

- Invent API. If the assembly cannot be written without a surface Longhorn does
  not have, that is a contract 020 gap and the milestone's planning checkpoint
  fires.
- Duplicate the contract. A guide that restates requirements drifts from them.

## Acceptance Criteria

- [x] an author who has not read the adapter source can assemble a window
- [x] every seam a GPUI application must implement appears in one list —
  eight, with the crate that owns each
- [x] the guide names at least one thing GPUI does worse than Tauri — three
  withheld capabilities, display facts the SDK will not supply, and a close
  decision that must be answered synchronously

## Evidence Required

- [x] the guide. Its example compiles under Card 174, which is the next card.
- [x] linked from the guides index, `getting-started.md`, and the Tauri
  composition guide

## Stop Conditions

Did not fire. The assembly is writable against surface that exists today.
