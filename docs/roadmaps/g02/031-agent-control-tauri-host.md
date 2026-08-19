# g02.031 Agent Control Tauri Host

Status: done 2026-08-19
Owner: Longhorn maintainers
Created: 2026-08-19
Governing refs: contract 022; contracts 010, 012, 020
Depends on: g02.030 complete — true 2026-08-19 (PR 3)

## Outcome

`longhorn-tauri-agent-control` exists: the Tauri plugin that mounts the
core server in dev builds, owns window operations, and captures unfocused
window images through the platform webview — with a release-build artifact
scan proving the entire surface is absent when the dev feature is off.

## Generation Runway

- [x] [Card 230](batch-cards/230-agent-control-tauri-plugin.md) — plugin,
      server mount behind the dev feature flag, localhost binding, window
      enumeration and targeting, release-absence artifact scan.
- [x] [Card 231](batch-cards/231-agent-control-capture.md) — WKWebView
      snapshot capture wired per the g02.029 evidence, including the
      recorded occluded/minimized behavior; packaged macOS
      unfocused-capture proof.

## Acceptance

- A release artifact contains no server, route, token, or discovery code.
- Capture works for an unfocused, occluded window on packaged macOS, with
  minimized-window behavior recorded honestly rather than claimed.
- The plugin adds no authority: it reaches app behavior only through
  existing command and IPC boundaries (contracts 006, 010).
