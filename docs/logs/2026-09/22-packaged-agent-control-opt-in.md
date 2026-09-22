# Packaged Agent-Control Opt-In

## Outcome

`longhorn-tauri-agent-control` ships the contract 022 server behind
`agent-control`, a compile-time opt-in a consumer may enable in a packaged
build. `evaluate` is a second feature, `agent-control-evaluate`, off by
default and expected only in dev/test. The default build stays an empty
library. The application still has to call `mount_agent_control`. Loopback,
the per-instance bearer token, and Origin validation are unchanged.

## Evidence

- Feature rename: Tauri host `dev` → `agent-control`. Core crate gains
  `agent-control-evaluate`; the Tauri feature forwards it.
- MCP `evaluate` stays listed. Without the evaluate feature it answers typed
  `Unsupported` and does not call the host handler. Semantic tools still
  marshal through the capture `eval_js` path; listen still reads the shim
  event ring through `ControlHandler::evaluate`.
- `check:agent-control-release-absence` proves three release artifacts
  (`longhorn.agent-control-release-absence.v2`): neither feature,
  `agent-control` only (server present, evaluate markers absent), both
  features (evaluate markers present). Isolated `-p` tests cover the
  Unsupported answer and the evaluate-on interleave fixture.
- Example proof enables both features. Composition guide, crate READMEs,
  install skill, and API surface name the packaged posture: loopback /
  token / Origin unchanged; the registered command catalogue is the allowed
  agency.

## Material limits

- Workspace `cargo test` unifies `agent-control-evaluate` from the proof
  example, so the Unsupported fixtures are `cfg(not(feature))` and run from
  the isolated absence scan, not from the workspace test graph.
- Capture, semantic tools, and `evaluate` remain macOS-only at the host
  (contract 020). Non-macOS still answers typed `Unsupported`.
