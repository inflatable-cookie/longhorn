# Agent Control Tauri Host

Date: 2026-08-19
Scope: g02.031 (Cards 230-231, PR 4)

## What happened

The third lane of the agent-control runway delivered
`longhorn-tauri-agent-control`: the Tauri host that composes the contract
022 server inside a running app, dev builds only. Mount from `setup`,
background-thread server with the core's token/bind/discovery lifecycle,
window scope over the app's webview windows, `command` through a
host-supplied bridge into the app's contract-006 registry, and the macOS
capture bridge (`WKWebView` snapshot + `evaluateJavaScript` escape hatch)
— all behind the off-by-default `dev` feature, with a repeatable
release-absence artifact scan wired into `qa` (and therefore the release
gate) proving the featureless artifact carries none of it.

Longhorn's own proof composition `examples/agent-control-proof` mounts
the plugin and carries the packaged freshness matrix: frontmost,
unfocused, occluded, minimized, and restored screenshots of the packaged
`.app` all pixel-fresh against `evaluate`-bracketed DOM counters — no
divergence from the Card 227 spike's debug-binary results, no permission
prompt, no entitlement, no private API. Clean quit removed the discovery
file on the real bundle.

## Evidence

- Plugin fixtures (`tests/mount.rs`, real loopback on a mock-runtime
  app): twelve tools listed, typed `Unsupported` for unwired tools,
  `command` through the bridge, window scope and typed unknown-window,
  401/403 guard paths, discovery published with the real port and removed
  on shutdown.
- Artifact scan `longhorn.agent-control-release-absence.v1`, both
  directions (feature-off: no graph edge, no core rlib, no markers;
  feature-on: positive control), rerun green with the capture code behind
  the feature.
- Packaged matrix `longhorn.agent-control-freshness-matrix.v1` in
  `examples/agent-control-proof/evidence/2026-08-19T17-41-52-packaged/`:
  five of five states fresh (brackets 1..1, 3..3, 5..5, 6..6, 8..8, each
  matched pixel-exact), `discoveryRemovedOnQuit: true`.
- `effigy qa` exit 0 in the worker worktree.

## Deviations and findings

- **Core fix, reported:** the Card 229 server served `type` as `r#type`
  (rmcp's tool macro takes the fn ident). Fixed with an explicit tool
  name and pinned by a core conformance test asserting the full wire
  vocabulary. No public-surface change.
- **`evaluate` wired in this lane:** the handoff listed it among the
  `Unsupported` answers, but Card 231's matrix judges freshness "by
  `evaluate` bracketing exactly as the spike did". Implemented as the raw
  `evaluateJavaScript` escape hatch through the capture bridge — host
  mechanics, not the g02.032 TS shim, which still owns the semantic
  tools.
- **Composition note for consumers:** a macOS quit delivers
  `RunEvent::Exit` without a preceding `ExitRequested`. Hooking only
  `ExitRequested` strands the discovery file on clean quit (found while
  the packaged matrix's lifecycle probe failed); the mount docs and the
  proof app hook both events.
- **Scan hardening:** the scan prunes prior rlibs of the two crates under
  test in its isolated target dirs, so a source change between runs
  cannot leave a stale second hash (caught by the first full `qa` run).
- **Non-macOS compile claim rests on the cfg construction** — only the
  macOS toolchain ran; recorded in the Card 231 closeout rather than
  overclaimed.

## Worker loop

Handoff
`docs/handoffs/20260819-172142-agent-control-tauri-host-worker.md`,
branch `worker/231-agent-control-tauri-host`, worktree
`/Users/tom/Dev/worktrees/longhorn-231` (manual fallback: the session
root was the planning checkout on `main`). Worktree bootstrap repeated
the 2026-08-16 papercut path (`bun install`, remove Bun's registry
symlinks, `effigy deps link bun ../poodle`, `POODLE_REPO` for proofs).
The packaged matrix used the operator's display for about a minute per
run, coordinated in-session.
