# 230 Agent Control Tauri Plugin

Status: ready
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

- [ ] plugin composes in a dev-featured build: token generated, server
      bound on 127.0.0.1, discovery file published with the real port and
      removed on clean exit
- [ ] an MCP client can list tools against the composed app;
      not-yet-wired tools answer typed `Unsupported`
- [ ] `command` reaches the contract 006 registry and returns its result
      through the core vocabulary
- [ ] a build without the dev feature contains no gated code, proved by
      the repeatable artifact scan, and the scan is runnable by the
      release gate
- [ ] no authority added: the plugin reaches app behavior only through
      existing command and IPC boundaries (contracts 006, 010)
- [ ] `effigy qa` passes

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

## Continuation

Card 231 wires capture and the packaged unfocused proof in the same lane.
