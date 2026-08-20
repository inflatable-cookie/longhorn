---
title: Figmatic child-webview screenshot composition handoff
kind: northstar-handoff
status: ready
owner: Tom
created: 2026-08-20
updated: 2026-08-20
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260820-142528-figmatic-child-webview-screenshot-composition.md
tags: [coordination, handoff, agent-control, tauri, native-content]
---

## What This Thread Was Doing

Figmatic is adopting Longhorn's dev-only agent-control surface in PR 14:

https://github.com/inflatable-cookie/figmatic/pull/14

The consumer integration follows
`/Users/tom/Dev/projects/longhorn/docs/guides/agent-control-composition.md`:
an off-by-default Cargo feature, `mount_agent_control`, `NoCommandBridge`, both
Tauri exit hooks, the installed skill, and a release-absence check. End-to-end
MCP verification proved discovery, the printed `claude mcp add` line,
`snapshot`, `click`, and `wait_for` against a real Figmatic window.

Figmatic then found an upstream screenshot gap. Longhorn main
`c1482daf830278ebd1162358f092f1926fac6224` keeps a Tauri window enumerable
after Figmatic attaches its preview child webview, but `screenshot` captures
only the same-label UI webview. The native preview island is an entirely black
rectangle in the returned PNG.

## Why It Matters

Figmatic's core working surface is the preview child webview. Agent control is
intended to replace OS computer use so agents can exercise and verify the app
without stealing focus or the pointer. A screenshot that omits the preview lets
an agent operate the shell but makes Componentize and visual acceptance
unobservable.

This also conflicts with Longhorn's current authority. Contract 022 says
native-content islands are visible in screenshots, and the composition guide
says child webviews are not semantic targets but appear in screenshots. The
consumer is relying on exactly that boundary; it is not asking for semantic
control of the child webview.

## Current State

- **Done in Longhorn:** `de979b80` / `c1482daf` fixed enumeration of windows
  hosting child webviews. The regression fixture proves that the parent window
  remains addressable.
- **Done in Figmatic:** PR 14 composes the plugin correctly, retains the
  Commands exclusion and its regression test, and proves the normal/release
  path contains no agent-control code.
- **Proved live:** The installed skill, finder, printed Claude MCP add command,
  connection, `snapshot`, `click`, and `wait_for` all work against Figmatic.
- **Blocked:** Longhorn `screenshot` does not composite the attached preview
  child webview into the window image.
- **Consumer evidence:**
  `/Users/tom/Dev/worktrees/figmatic-g01-013-agent-control/target/tmp/agent-control-proof.png`
  is a 3600x2260 capture with the native preview island black.
- **Figmatic record:**
  `/Users/tom/Dev/worktrees/figmatic-g01-013-agent-control/docs/logs/2026-08/20-121156-g01-013-review-corrections.md`
  and
  `/Users/tom/Dev/worktrees/figmatic-g01-013-agent-control/docs/roadmaps/g01/013-dev-agent-control.md`.
- **Canonical review:**
  https://github.com/inflatable-cookie/figmatic/pull/14#issuecomment-5356274555
- **Longhorn authority:**
  `/Users/tom/Dev/projects/longhorn/docs/contracts/022-agent-app-control.md`
  and
  `/Users/tom/Dev/projects/longhorn/docs/guides/agent-control-composition.md`.
- **Likely implementation surfaces:**
  `/Users/tom/Dev/projects/longhorn/crates/longhorn-tauri-agent-control/src/capture.rs`,
  `/Users/tom/Dev/projects/longhorn/crates/longhorn-tauri-agent-control/src/handler.rs`,
  and
  `/Users/tom/Dev/projects/longhorn/crates/longhorn-tauri-agent-control/src/mount.rs`.
- **Roadmap state:** g02.029-033 are complete. Treat this as new Figmatic
  adoption evidence and compile a bounded follow-up through Longhorn's docs
  spine; do not silently reopen a completed card.

## Boundaries

- **In scope:** Make the Tauri agent-control `screenshot` result show the whole
  Figmatic window, including attached child-webview/native-content islands;
  add regression and live evidence; return an exact Longhorn revision for the
  Figmatic adoption rerun.
- **Not required:** Semantic snapshot targets, clicks, typing, or commands
  inside the preview child webview. Contract 022 deliberately limits it to
  screenshot visibility.
- **Out of scope:** A Figmatic-side capture workaround, adopting app-specific
  policy into Longhorn, weakening the dev-only/release-absence boundary, or
  redefining a black island as successful capture.
- Preserve fresh capture while the app is unfocused, occluded, or minimized.
  Do not regress the no-focus/no-pointer-theft property.
- Do not introduce private APIs, screen-recording permission, or OS desktop
  capture without first promoting the resulting contract and security choice.
- Follow `/Users/tom/Dev/projects/longhorn/AGENTS.md`. If public Tauri/WebKit
  APIs cannot satisfy the existing promise, stop with evidence and bring the
  contract gap back to the operator rather than improvising around it.

## Important Context

The current failure is narrower than enumeration. A Tauri window can own a
same-label main webview plus one or more differently labelled child webviews.
Longhorn now discovers the parent through the `Window` plus same-label-webview
walk, so MCP tools can target it. Capture still snapshots only the UI webview,
which does not include sibling WKWebView pixels.

The likely design question is how to produce one truthful window image from
the parent and its child webviews while retaining correct physical/logical
bounds, scale, clipping, and z-order. Per-webview snapshots composed by bounds
may be viable, but that is a hypothesis to test, not a prescribed design. A
different public-API route is acceptable if it meets the same contract.

Do not confuse this with Figmatic's native-content visibility policy. The
preview was visibly present in the real window during the proof; only the MCP
PNG was black. The child view is intentionally not part of the main DOM and
cannot be recovered from the semantic snapshot.

The Figmatic shutdown proof is already complete. A manually focused Cmd+Q
exercised `RunEvent::Exit`, removed discovery, and confirmed the second exit
hook. The previously observed dead-pid discovery file came from a watcher-killed
diagnostic process and was removed. Do not reopen that issue unless new evidence
appears.

## Suggested Next Move

Start by reproducing the black-island capture from the artifact and Figmatic PR
14, then isolate it in a minimal Longhorn fixture: one parent Tauri window with
distinct visible content in the main webview and an attached, differently
labelled child webview. The fixture should fail unless both surfaces appear in
the returned screenshot.

Characterize the supported Tauri/WebKit capture mechanisms before freezing the
implementation. Then implement the smallest consumer-neutral composition path
that preserves:

- one image for the complete logical window;
- correct child bounds at 1x and 2x scale;
- clipping at the parent viewport;
- deterministic ordering for multiple overlapping children;
- fresh capture when unfocused, occluded, and minimized; and
- unchanged token, window-selection, size-cap, and release-absence behavior.

Add focused unit/integration coverage plus a packaged macOS proof. Once the
Longhorn fix lands, pull that exact revision into the Figmatic PR 14 worktree,
rerun the installed skill's `snapshot -> click -> wait_for -> screenshot`
sequence, and confirm that the real preview content is present in the saved
PNG. Figmatic owns its dependency update and final merge.

## Completion Protocol

1. Characterize and promote the new adoption finding through Longhorn's docs
   spine, then compile a bounded roadmap card or explicitly record why the
   existing contract already authorizes a direct correction.
2. Prove a fixture containing both parent and child webview content. Include
   scale, bounds, clipping, and ordering cases proportional to the chosen
   implementation.
3. Run focused checks during development, then `effigy qa` for the completed
   batch and `effigy check:agent-control-release-absence` explicitly if QA does
   not expose that evidence clearly.
4. Run the packaged live proof without moving OS focus or the pointer. Record
   any state where capture freshness cannot be preserved.
5. Update contract 022, the composition guide, roadmap state, and a dated log
   so the claimed screenshot boundary matches the mechanism actually proved.
6. Return a reviewable PR from a dedicated non-`main` worktree. Do not merge it
   from the worker thread.
7. Give the Figmatic orchestrator the landed Longhorn commit and exact rerun
   instructions. Completion requires a new Figmatic MCP screenshot whose
   preview island contains the rendered page rather than black pixels.

If public APIs cannot produce that image without screen-capture permission or
focus theft, stop with the minimal failing fixture and the available design
choices. That is an operator-level contract decision, not permission to weaken
the evidence.
