# Agent File Input

2026-09-26. [g02.048](../../roadmaps/g02/048-agent-file-input.md) merged
through [PR #38](https://github.com/inflatable-cookie/longhorn/pull/38) at
`a25b3fe6` (worker head `57c1619e`). Queue task
`e0080ad6-9ac9-48ca-8196-ef22cc165b29` closed at `d2e481f9`. Independent review
passed on the first head.

## What landed

- `set_file_input { element, files: [{ name, mediaType?, contentBase64 }],
  window?, webview? }` in the agent-control catalogue. The agent is the byte
  source: no path field (`deny_unknown_fields`), no filesystem read, no OS
  panel. Non-empty names, standard base64, at least one file, decoded total
  at most 8 MiB.
- The shim resolves the ref, requires `<input type="file">`, checks
  `multiple` and `accept` (extensions, media types, wildcards), assigns a
  constructed `DataTransfer` `FileList`, and dispatches untrusted `input` and
  `change`. Agent origin is marked. Same child-webview targeting as
  `click`/`type`; a packaged `agent-control` build answers it without
  `agent-control-evaluate`.
- Composition guide, crate READMEs, and skill table updated.

## Evidence

- WKWebView probe (`callAsyncJavaScript`): assigning `files` from a constructed
  `DataTransfer` works; `file.text()` returns the supplied content.
- Shim fixtures: files set, events fire, content, name, and type preserved;
  typed failures for non-file target, stale ref, `multiple`, `accept`, bad
  base64, empty list.
- Rust: empty list, empty name, bad base64, over 8 MiB fail; exactly 8 MiB
  passes; MCP args reject `path`. Conformance lists the tool.
- `check:agent-control-skill`, `check:agent-control-release-absence`
  (`setFileInput` present with the feature, absent without), and `effigy qa`
  green; the reviewer reran all three.

## Limits

- The 8 MiB cap is enforced at the MCP edge (`SetFileInputArgs::into_request`),
  not again in `TauriControlHandler::set_file_input`. MCP and the stdio
  carrier are the only external routes, so contract 022 holds on the wire; a
  direct in-process `ControlHandler` caller would skip the cap. See
  `PAPERCUTS.md`.
- No dedicated closed-child-webview fixture; the shared `eval_js` targeting
  covers it.
- The Tauri host carries a hand-synced shim copy
  (`agent_control_shim.js`) with no drift check. The reviewer compared the new
  functions by hand. Pre-existing; see `PAPERCUTS.md`.
- Soundcheck manifest-compare adoption is consumer-owned and pending.
- Unreleased. Ships with the next Longhorn version.
