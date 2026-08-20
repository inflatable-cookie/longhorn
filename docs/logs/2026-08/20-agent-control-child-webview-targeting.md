# Agent Control Child-webview Targeting

Date: 2026-08-20
Scope: g02.035 (Cards 239-240, PR pending); contract 022 opt-in rule;
Figmatic preview-input handback

## What happened

Figmatic's preview island was screenshot-visible after g02.034, but
semantic and input tools still stopped at the UI webview. The operator
made the product call: agents driving the preview is the feature. Cards
239-240 implement the 2026-08-20 contract amendment: an app may opt in
named child-webview labels at mount; opted-in children are full semantic
targets; default stays closed.

Card 239 is addressing and refusals, not new machinery. The shim was
already injected into every webview; evaluate already took a per-webview
handle; the handler already walked `Window` + `webviews()`. What landed:
`AgentControlConfig::with_semantic_child`, an additive `webview` field on
semantic/input tools, `UnknownWebview` for a label that matches nothing,
`Unsupported` naming opt-in absence for a hosted-but-closed child, and
DOM-prefixed refs so two webviews' `eN` strings cannot cross-hit.

Ref scoping does not keep a server-side table. UI refs stay `eN`. Child
refs are `{encodeURIComponent(label)}:eN`. Routing is the request's
`webview` target. Collision is fixtured in the shim (same local seq, two
documents, `UnresolvedRef` both ways) and re-proved packaged.

## Evidence

- Plugin fixture `child_webview_targeting_refusals_and_opt_in`: late
  attach after mount, unknown label, closed child, opted-in child found.
- Shim collision fixture and the IIFE byte-lock.
- Packaged e2e v2
  (`examples/agent-control-proof/evidence/2026-08-20T15-55-00-e2e/`):
  island snapshot → click → type (`Marquee`) → drag (Cell 0 0 → Cell 2 2
  selected `0,0:2,2`) → wait_for title Ready → evaluate, composed
  screenshot, `preview-top` `Unsupported`, island ref clicked on the UI
  webview `UnresolvedRef`, UI/island interleave with distinct refs. Five
  focus samples, app never frontmost.

## Drag, precisely

Untrusted `drag` is ref-to-ref and two-point (source center → target
center): pointer/mouse down-move-up plus HTML5 DnD. It does not
interpolate a pixel path. The packaged marquee listens to those events
and was driven. A free-form marquee that only samples intermediate
`mousemove` coordinates is not expressed.

## Handback

Figmatic owns the bump, the opt-in, and its preview-acceptance
automation. After merge, pin the landed `main` commit, bump
`longhorn-tauri-agent-control`, `.with_semantic_child("figmatic-preview")`
at mount, re-run the skill installer, drive the preview with
`webview: "figmatic-preview"`. Do not opt in `longhorn-browser` views.

## Worker loop

Handoff
`docs/handoffs/20260820-163602-agent-control-webview-targeting-worker.md`,
branch `t3code/read-webview-targeting-handoff`, worktree
`/Users/tom/.t3/worktrees/longhorn/t3code-3feafabe` (launcher-provided).
One packaged display run on the operator's display.
