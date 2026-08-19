# Agent Control Proof

Longhorn's own proof composition for the contract 022 control surface
(Cards 230-231, milestone g02.031): a minimal Tauri app that mounts
`longhorn-tauri-agent-control` behind its `dev` feature. This app exists to
prove the dev control surface; it is never shipped, and the feature is what
keeps every line of that surface out of release artifacts (proven by
`effigy check:agent-control-release-absence`).

The page renders a once-a-second counter whose value is also encoded in the
background hue — the freshness oracle the packaged matrix judges against.

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
