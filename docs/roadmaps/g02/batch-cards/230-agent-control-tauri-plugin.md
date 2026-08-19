# 230 Agent Control Tauri Plugin

Status: done 2026-08-19
Owner: Longhorn maintainers
Roadmap: g02.031
Governing refs: contract 022; contracts 010, 012, 020; memo 024
Depends on: Cards 228-229 (merged, PR 3)
Auto-start next card: yes — Card 231 in the same lane

## Objective

`longhorn-tauri-agent-control` exists: the Tauri plugin that composes the
core crate's server inside a running app, dev builds only, with proof that
release artifacts contain none of it.

## Scope

- **Plugin crate.** `longhorn-tauri-agent-control` following the existing
  `longhorn-tauri-*` pattern: a Tauri plugin that, when composed, generates
  the instance token, binds the core server on 127.0.0.1:0, publishes the
  contract 022 discovery file, and tears both down on app exit (discovery
  removal on clean shutdown; the stale-pid path covers crashes).
- **Dev-only gating.** The entire surface sits behind a cargo feature that
  is off by default. Gating is compile-time: a release build without the
  feature must contain no server, route, token, or discovery code. No
  runtime toggle can enable it (contract 022).
- **Handler wiring, window scope.** Implement the core `ControlHandler`
  for the Tauri host: window enumeration (`list_windows`), per-window
  targeting and `resize_window`, and `command` invocation through the
  contract 006 registry. Webview-dependent tools (`snapshot`, input,
  `evaluate`, `wait_for`, `screenshot`) land in Card 231 and the g02.032
  shim; until then they answer the core vocabulary's typed `Unsupported`
  error, not a panic and not a stub success.
- **Release-absence artifact scan.** A repeatable check (an `effigy`
  selector or a test that builds without the feature) proving the release
  artifact carries none of the gated surface: no server symbols, no
  discovery path string, no token code. Wire it so the release gate can
  run it, not as a one-off note.

Workspace admission per contract 012. No consumer composes the plugin in
this card; Longhorn's own proof composition arrives with Card 231.

## Acceptance Criteria

- [x] plugin composes in a dev-featured build: token generated, server
      bound on 127.0.0.1, discovery file published with the real port and
      removed on clean exit
- [x] an MCP client can list tools against the composed app;
      not-yet-wired tools answer typed `Unsupported`
- [x] `command` reaches the contract 006 registry and returns its result
      through the core vocabulary
- [x] a build without the dev feature contains no gated code, proved by
      the repeatable artifact scan, and the scan is runnable by the
      release gate
- [x] no authority added: the plugin reaches app behavior only through
      existing command and IPC boundaries (contracts 006, 010)
- [x] `effigy qa` passes

## Validation

`effigy qa`; the artifact scan run both ways (feature on: surface present;
feature off: absent); `effigy doctor`.

## Stop Conditions

- compile-time gating cannot fully exclude the surface from a featureless
  build (feature unification pulling the core crate's server into a
  release artifact counts) — stop and report before weakening to runtime
  gating;
- command invocation cannot go through contract 006 without new authority
  — that is a contract question, not plugin code.

## Closeout

Status: done 2026-08-19, on `worker/231-agent-control-tauri-host` in
worktree `/Users/tom/Dev/worktrees/longhorn-231` (manual fallback — the
session root was the planning checkout on `main`).

**Composition shape.** No `tauri::plugin::Builder` — the workspace has
none; the crate follows the actual `longhorn-tauri-*` pattern: a host
assembly the app calls from `setup` (`mount_agent_control`) plus a
shutdown handle the run-event callback drives. The server runs on a
background thread with its own tokio runtime; the core's
`serve_control_surface` owns token, bind, discovery publish, and
clean-exit removal. `CommandBridge` is the host-supplied seam into the
app's contract-006 registry — the plugin holds no command authority, so
the second stop condition did not trigger.

**Gating.** Everything — the core crate, `tauri`+`wry`, tokio, and (Card
231) the objc2 capture bridge — is optional behind the off-by-default
`dev` feature; feature-off the crate compiles to an empty library.
Feature unification pulling the core crate in is caught by the scan's
`cargo tree` assertion, so the first stop condition is guarded, not just
untriggered.

**Artifact scan.** `scripts/verify-agent-control-release-absence.ts`,
wired as `check:agent-control-release-absence` in `qa` (which the release
`workspace` gate runs — no separate release.gates line, per
config/release.toml's single-table rule). Both directions: feature-off
asserts no core-crate graph edge, no core rlib, no gated markers in the
plugin rlib; feature-on is the positive control proving the markers are
live. Receipt: `longhorn.agent-control-release-absence.v1`.

**One core fix (reported).** The Card 229 server served `type` under its
raw identifier `r#type`; the tool macro takes the name from the fn ident.
Fixed with an explicit `name = "type"` in the core's `server/mcp.rs` and
pinned by a new core conformance test asserting the full wire vocabulary.
No public-surface change — the vocabulary already said `type`.

**Evidence.** `crates/longhorn-tauri-agent-control/tests/mount.rs` (2
fixtures, real loopback against a mock-runtime app): tools/list returns
all twelve tools; `snapshot` answers typed `unsupported`; `command`
reaches the bridge and returns its output; `list_windows` /
`resize_window` / unknown-window typed failure; unauthenticated → 401,
foreign `Origin` → 403, loopback origin admitted; discovery published
with the real port and removed on `shutdown()`. The packaged proof app
(Card 231) re-proves composition, `command` against a real contract-006
registry, and clean-exit discovery removal on a real bundle.

**Versions (workspace lock):** tauri 2.11.5, tauri-runtime 2.11.3 (the
workspace's caret `2.10.3` resolves here — the spike's `=2.10.3` pin was
prototype-local; no workspace pin moved), rmcp 3.1.3, axum 0.8.9, tokio
1.53.1.

**Found while wiring (also in Card 231's closeout):** a macOS quit
delivers `RunEvent::Exit` without a preceding `ExitRequested`; the mount
docs and the proof app hook both events, and the composition docs say so.

## Continuation

Card 231 wires capture and the packaged unfocused proof in the same lane.
