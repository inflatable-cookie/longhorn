# 238 Agent Control Screenshot Composition

Status: ready
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

- [ ] baseline black-island capture committed before the fix
- [ ] both-surfaces fixture fails on the old path, passes on the new
- [ ] scale (1x/2x), parent-viewport clipping, and overlap-order cases
      proved proportional to the design
- [ ] packaged freshness matrix green with the child attached, every
      window state, DOM-relative judgment per webview
- [ ] token/window-selection/release-absence unchanged; the
      release-absence scan green both directions
- [ ] contract 022 and the guide match the proved mechanism exactly
- [ ] closeout names the revision and Figmatic rerun steps
- [ ] `effigy qa` passes

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

## Continuation

g02.034 closes with this card. Figmatic owns its dependency bump, PR 14
rerun, and merge; the orchestrator relays the revision.
