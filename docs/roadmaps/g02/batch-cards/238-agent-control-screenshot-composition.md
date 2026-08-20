# 238 Agent Control Screenshot Composition

Status: done 2026-08-20
Owner: Longhorn maintainers
Roadmap: g02.034
Governing refs: contract 022 (its screenshot claim is what this card
makes true or narrows); the Figmatic handoff
`docs/handoffs/20260820-142528-figmatic-child-webview-screenshot-composition.md`
(read it in full — it is the finding's primary record); contracts 001,
012, 020; Card 231's capture mechanics
Depends on: `c1482daf` (multiwebview enumeration)
Auto-start next card: no — single-card milestone

## Objective

A `screenshot` of a window hosting child webviews shows every webview's
real pixels, not black where siblings render. Contract 022 already
promises this ("native-content islands appear in screenshots"); the
mechanism — `takeSnapshot` on the UI webview alone — cannot deliver it.
Make the mechanism match the claim, or bring back the evidence that no
public-API mechanism can.

## Scope

- **Reproduce first.** Extend `examples/agent-control-proof` with an
  attached child webview carrying visibly distinct content (its own hue
  ticker with a different stride works with the existing evidence
  method). Before any fix, capture the black-island failure — that PNG
  is the baseline evidence.
- **Characterize before freezing.** Survey the supported capture
  mechanisms (per-webview `takeSnapshot` composed by bounds is the
  leading hypothesis, not the prescribed design; another public-API
  route meeting the same contract is acceptable). Record what was
  considered and why the winner won — Card 226's lesson applies: read
  the platform before building the elaborate version.
- **Composition invariants** (from the Figmatic handoff, verbatim
  intent): one image for the complete logical window; correct child
  bounds at 1x and 2x scale; clipping at the parent viewport;
  deterministic ordering for overlapping children; fresh capture
  unfocused, occluded, and minimized; token, window-selection,
  size-cap, and release-absence behavior unchanged.
- **Fixture.** A mount-level or packaged fixture that fails unless both
  the UI webview's and the child's content appear in the returned PNG,
  plus cases proportional to the chosen design (scale, clipping,
  overlap order). The mock runtime cannot render; expect the pixel
  proofs to live in the packaged driver like Cards 231/234.
- **Packaged proof.** Re-run the freshness matrix on the extended proof
  app with the child attached — both surfaces fresh in every state,
  judged DOM-relative by each webview's own bracket.
- **Truthfulness pass.** Contract 022 and the composition guide say
  exactly what the mechanism now proves. If a state cannot be composed
  (e.g. a child's snapshot goes stale minimized where the parent's does
  not), that is a recorded narrowing, not a silent hole.
- **Handback.** Card closeout names the exact revision and the rerun
  steps for Figmatic PR 14; a dated log entry records the lane.

## Acceptance Criteria

- [x] baseline black-island capture committed before the fix —
      `examples/agent-control-proof/evidence/2026-08-20T13-53-08-packaged/`
      (in this composition the island region carried the parent's pixels
      rather than black, the parent's background being opaque; the island's
      own content was absent either way, and the matrix judged it failing)
- [x] both-surfaces fixture fails on the old path, passes on the new —
      same matrix: failing receipt at the baseline commit, green at the fix
- [x] scale (1x/2x), parent-viewport clipping, and overlap-order cases
      proved proportional to the design — 2x proved pixel-exactly (edge,
      clip, overlap, hidden-region probes); 1x recorded as unprobed in
      contract 022's narrowings, no 1x display available
- [x] packaged freshness matrix green with the child attached, every
      window state, DOM-relative judgment per webview —
      `examples/agent-control-proof/evidence/2026-08-20T14-14-36-packaged/`
- [x] token/window-selection/release-absence unchanged; the
      release-absence scan green both directions
- [x] contract 022 and the guide match the proved mechanism exactly
- [x] closeout names the revision and Figmatic rerun steps
- [x] `effigy qa` passes

## Validation

`effigy qa`; `check:agent-control-release-absence` explicitly if qa
does not surface it; the packaged proof on a macOS host (operator's
display, no focus or pointer theft).

## Stop Conditions

- No public-API route produces the composed image without
  screen-recording permission, private API, or focus theft — stop with
  the minimal failing fixture and the design choices; that is an
  operator-level contract decision, not permission to weaken evidence.
- The composition wants app-specific policy (which children to include,
  island visibility rules) — that is consumer authority; the mechanism
  composes what the window actually hosts, nothing smarter.
- A redefinition of the black island as acceptable output — explicitly
  forbidden by the handoff; do not.

## Closeout

Status: done 2026-08-20, branch `worker/238-agent-control-composition`,
worktree `/Users/tom/Dev/worktrees/longhorn-238` (manual fallback; the
session root was the planning checkout on `main`).

**Mechanisms considered.** Per-webview `takeSnapshot` composed by bounds
won. `cacheDisplayInRect:toBitmapImageRep:` on the window's content view
was rejected without a build: it is deprecated, synchronous, and its known
behavior with out-of-process composited layers (blank webview content) is
exactly the failure class `takeSnapshot` exists to solve; the baseline
fixture already demonstrated that view-level rendering does not reach
sibling `WKWebView` pixels. `CGWindowListCreateImage`/ScreenCaptureKit
need screen-recording permission and capture the screen, not the window —
excluded by the handoff boundaries. `WKSnapshotConfiguration` with a rect
is the same family as the nil-config viewport snapshot used; the viewport
is the wanted region per webview.

**Chosen design.** `screenshot_window` in the plugin's capture bridge:
each hosted webview snapshots its own viewport (nil config), drawn into a
physical-pixel bitmap of the window's inner size — the UI webview at the
origin, children at their tauri-reported physical position/size, back to
front in the z-order sampled from the view hierarchy at capture time
(label order breaks ties), clipped by the bitmap. Hidden webviews are
omitted; a snapshot failure on any hosted visible webview fails the call
typed. All AppKit drawing stays on the main thread; only PNG bytes cross
threads.

**Findings.** (1) The baseline failure mode in Longhorn's proof was
parent-pixels-over-island, not black — the parent's opaque background
fills the region the island covers; Figmatic's black came from its own
page. Same contract gap either way. (2) The 1x/2x trap is real and was
hit: a 2x snapshot decodes to an `NSBitmapImageRep` whose *point* size is
half its pixel count, so a source rect built from pixel counts draws the
image into a corner quarter; the fix spans the source rect in the rep's
own coordinate space. (3) Orientation facts (bitmap row 0 is the PNG top;
an unflipped bitmap context draws y-up with upright content) were pinned
by a live scratch probe before the implementation — Card 226's
read-the-platform-first lesson applied.

**Narrowings.** Recorded in contract 022: composition proved at 2x only
(no 1x display available; the composition is physical-pixel explicit
throughout), and genuinely native non-webview surfaces still do not
appear in the image. Child-webview freshness held in every probed state
including minimized, so no per-state narrowing was needed.

**Evidence.** Baseline (failing):
`examples/agent-control-proof/evidence/2026-08-20T13-53-08-packaged/`.
Green matrix v2 (five states × parent+island fresh, geometry probe
pixel-exact at 2x, overlap order and hidden-absence proved):
`examples/agent-control-proof/evidence/2026-08-20T14-14-36-packaged/`.

**Figmatic rerun.** The revision under review is the PR head of
`worker/238-agent-control-composition`; after merge, pin the landed
`main` commit. In the Figmatic PR 14 worktree: bump the
`longhorn-tauri-agent-control` dependency to that revision, rebuild,
launch, and rerun the installed skill's `snapshot` → `click` → `wait_for`
→ `screenshot` sequence against a real Figmatic window with the preview
attached. Completion per the Figmatic handoff: the new MCP screenshot's
preview island contains rendered content, not black. Figmatic owns its
dependency bump, PR 14 rerun, and merge.

## Continuation

g02.034 closes with this card. Figmatic owns its dependency bump, PR 14
rerun, and merge; the orchestrator relays the revision.
