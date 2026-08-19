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
counter's hue. It finishes by quitting the app cleanly and asserting the
discovery file is removed. Evidence (PNGs plus `matrix.json`) lands in
`evidence/<timestamp>-packaged/` and is recorded in the Card 231 closeout.

It needs the operator's display for about a minute and opens one Terminal
window (closed afterwards); coordinate the run rather than assuming the
desktop is free.

## Running the packaged end-to-end driver (macOS, operator's display)

```sh
bun examples/agent-control-proof/e2e.ts
```

The driver builds the `.app`, launches it unfocused (`open -g`), and runs
snapshot → click → type → wait_for → screenshot → command over a real MCP
connection, plus two-client snapshot interleave and two listen streams.
It records that the app never held OS focus (System Events) and that no OS
pointer motion exists in the path. Evidence lands in
`evidence/<timestamp>-e2e/`.
