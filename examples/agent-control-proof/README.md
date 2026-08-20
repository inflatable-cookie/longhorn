# Agent Control Proof

Longhorn's own proof composition for the contract 022 control surface
(Cards 230-234): a consumer-shaped Tauri app that mounts
`longhorn-tauri-agent-control` behind its `dev` feature. This app exists to
prove the dev control surface; it is never shipped, and the feature is what
keeps every line of that surface out of release artifacts (proven by
`effigy check:agent-control-release-absence`).

The page keeps the Card 231 hue-encoded ticker and adds a small form-and-list
UI (click, type, drag-reorder, hash navigation) so an agent can drive it
unfocused. Synthetic input is untrusted DOM events: it never moves the OS
pointer and never holds focus.

Card 238 attaches three child webviews to the main window — the
native-content-island shape from the Figmatic adoption finding: the
`preview` island (`island.html`, 97° hue stride), deliberately oversized so
its right and bottom edges clip at the window viewport; `preview-top`
(`island-top.html`, 199° stride) overlapping it, attached later so the
composed screenshot must show it on top; and `preview-hidden`, hidden after
attach so its region must show the parent page. Card 239 opts `preview` in
as a semantic target (click, type, two-point drag, wait_for, evaluate);
`preview-top` stays closed so the packaged refusal has a live counterpart.

Its contract-006 registry (`proof:ping`, `proof:window.minimize`,
`proof:window.restore`) exists so the matrix scripts window state through
the same `command` tool an agent uses, proving the registry path end to end.

## Running the packaged freshness matrix (macOS, operator's display)

```sh
cd examples/agent-control-proof
bunx @tauri-apps/cli build
cd ../..
bun examples/agent-control-proof/freshness-matrix.ts
```

The driver launches the packaged `.app`, reads the discovery file, and
probes frontmost, unfocused, occluded (a Terminal window over the app),
minimized, and restored states — each screenshot bracketed by `evaluate`
counter reads, judged fresh when the captured pixels match a bracketed
counter's hue. With the islands attached, every row judges both surfaces:
the parent's region against its own bracket, the base island's region
against the island's encoding over the parent's bracket widened by two
ticks (all tickers start at page load at 1 Hz; the matrix still reads the
island counter from the PNG so the composition judgment stays
pixel-relative). A frontmost-only
geometry probe checks the base island's left and top edges pixel-exactly,
that the overlap region shows the later-attached island, and that the
hidden island's region shows the parent page, and records the PNG
dimensions and observed scale factor. It finishes by quitting the app
cleanly and asserting the discovery file is removed. Evidence (PNGs plus
`matrix.json`) lands in `evidence/<timestamp>-packaged/`.

It needs the operator's display for about a minute and opens one Terminal
window (closed afterwards); coordinate the run rather than assuming the
desktop is free.

## Running the packaged end-to-end driver (macOS, operator's display)

```sh
bun examples/agent-control-proof/e2e.ts
```

The driver builds the `.app`, launches it unfocused (`open -g`), and runs
snapshot → click → type → wait_for → screenshot → command over a real MCP
connection, plus two-client snapshot interleave and two listen streams,
then the Card 240 island drive (the same six motions inside opted-in
`preview`), the closed-child `Unsupported` and cross-webview
`UnresolvedRef` legs, and UI/island interleave. It records that the app
never held OS focus (System Events) and that no OS pointer motion exists
in the path. Evidence lands in `evidence/<timestamp>-e2e/`.

## Running the skill-only dogfood pass (macOS, operator's display)

```sh
bunx @tauri-apps/cli build
cd ../..
bun examples/agent-control-proof/dogfood.ts
```

This follows the composition guide to launch the packaged app, then the
canonical skill (`skills/agent-control/SKILL.md`) for finder, raw-POST
connection, and snapshot → click → type → wait_for → screenshot → command
→ listen. Evidence lands in `evidence/<timestamp>-skill-dogfood/`. The
token is redacted; the app must stay unfocused.
