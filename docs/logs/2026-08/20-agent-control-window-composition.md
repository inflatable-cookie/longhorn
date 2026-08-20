# Agent Control Window Composition

Date: 2026-08-20
Scope: g02.034 (Card 238, PR pending); contract 022 screenshot claim;
Figmatic handoff closeout

## What happened

Figmatic's adoption (its PR 14) found contract 022's first
mechanism-vs-contract gap: `screenshot` snapshotted only the UI webview,
so an attached child webview — Figmatic's preview island, the surface the
app exists to produce — came back black. Card 238 made the mechanism
match the contract's claim.

Sequence held per the handoff: baseline first, mechanisms second,
implementation third. The extended proof app (three child webviews: a base
island clipped at the viewport, an overlapping island attached later, a
hidden island) reproduced the failure on the old path — locally the island
region carried the parent's pixels, not black, because the parent's
background is opaque; the contract gap is identical. The failing receipt
and PNGs are committed as the baseline
(`examples/agent-control-proof/evidence/2026-08-20T13-53-08-packaged/`).

The mechanism: each hosted webview snapshots its own viewport
(`takeSnapshot` reaches only the webview it is called on), drawn into a
physical-pixel bitmap of the window's inner size — UI webview at the
origin, children at tauri-reported physical bounds, back to front in the
view hierarchy's sampled z-order, clipped by the bitmap, hidden webviews
omitted, any visible webview's snapshot failure failing the call typed.
Whole-contentView `cacheDisplayInRect:` was rejected on its known blank
behavior with out-of-process webview layers; ScreenCaptureKit needs
screen-recording permission and was out of bounds.

## Evidence

- Green packaged matrix v2
  (`examples/agent-control-proof/evidence/2026-08-20T14-14-36-packaged/`):
  five window states (frontmost, unfocused, occluded, minimized, restored),
  parent and island both fresh per state, judged DOM-relative (the island's
  counter read from the PNG against the parent's `evaluate` bracket, both
  tickers 1 Hz from page load).
- Geometry probe, frontmost, pixel-exact at 2x: parent hue one logical
  pixel left/above the island edges, island hue one pixel right/below; the
  overlap region names the later-attached island; the hidden island's
  region shows the parent page; PNG dimensions record the 2x scale.
- Plugin fixtures green, including the multiwebview enumeration fixture
  extended to assert typed screenshot failure on the mock runtime.
- `effigy qa` green in the worker worktree.

## Deviations and findings

- **The 1x/2x trap was hit, not just theorized.** A 2x snapshot decodes to
  an `NSBitmapImageRep` whose point size is half its pixel count; a source
  rect built from pixel counts drew each surface into a corner quarter of
  its destination. The fix spans the source rect in the rep's own
  coordinate space (`rep.size()`), making the composition correct at any
  backing scale by construction.
- **Orientation was probed, not assumed** (Card 226's lesson): a scratch
  harness pinned that bitmap row 0 encodes to the PNG top and that an
  unflipped bitmap context draws y-up with upright content, before the
  compositor was written.
- **Baseline failure mode differed from Figmatic's** (parent pixels, not
  black) — recorded in the card closeout so the evidence reads honestly.
- **Core-crate doc comments left untouched** (report-first seam): the
  `ScreenshotRequest` and MCP tool descriptions read true as written
  ("window image via webview capture"); a follow-up wording tweak is
  flagged in the PR rather than edited here.
- The freshness matrix receipt schema bumped to v2 (island judgment,
  geometry block). v1 evidence from Card 231 remains in the tree.

## Limits

- Proved at 2x backing scale only; no 1x display was available. Recorded
  in contract 022's narrowings.
- Genuinely native (non-webview) surfaces still do not appear in the
  composed image; no provider ships.
- Another-Space remains unprobed (Card 231's standing limit).

## Handback

Figmatic owns its dependency bump, PR 14 rerun, and merge. Rerun steps in
the Card 238 closeout; completion per the Figmatic handoff is a new MCP
screenshot whose preview island contains rendered content.

## Worker loop

Handoff
`docs/handoffs/20260820-143033-agent-control-composition-worker.md`,
branch `worker/238-agent-control-composition`, worktree
`/Users/tom/Dev/worktrees/longhorn-238` (manual fallback: the session root
was the planning checkout on `main`). Three packaged display runs on the
operator's display (baseline, one mid-fix iteration that caught the
point-vs-pixel bug, green), about a minute each.
