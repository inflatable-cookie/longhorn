# g02.034 Agent Control Window Composition

Status: ready
Owner: Longhorn maintainers
Created: 2026-08-20
Governing refs: contract 022; contracts 001, 012, 017 (island
coordination context), 020; Figmatic handoff
`docs/handoffs/20260820-142528-figmatic-child-webview-screenshot-composition.md`
Depends on: g02.029-033 complete; the `c1482daf` multiwebview
enumeration fix

## Outcome

`screenshot` returns one truthful image of the whole logical window —
parent UI webview and attached child webviews composed at correct
bounds, scale, clipping, and order — or contract 022's screenshot claim
is narrowed honestly with the failing fixture as evidence. Figmatic's
preview island stops being a black rectangle.

## Generation Runway

- [ ] [Card 238](batch-cards/238-agent-control-screenshot-composition.md)
      — fixture, mechanism characterization, composition path, packaged
      proof, contract/guide truthfulness, Figmatic rerun handback.

## Acceptance

- A fixture window with distinct visible content in both the UI webview
  and a differently labelled child fails unless both appear in the
  returned PNG.
- Freshness (unfocused, occluded, minimized), token, window selection,
  and release absence are unchanged.
- Contract 022 and the composition guide state exactly what the
  mechanism proves — no implied claims either direction.
- Figmatic receives an exact Longhorn revision and rerun instructions;
  completion includes its preview rendering in a real MCP screenshot.

## Next Task

After completion: Figmatic PR 14 reruns on the landed revision; the
remaining three consumer adoptions continue.
