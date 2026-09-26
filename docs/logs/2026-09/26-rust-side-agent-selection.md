# Rust-side Agent Selection

2026-09-26. [g02.047](../../roadmaps/g02/047-rust-side-agent-selection.md)
merged through [PR #36](https://github.com/inflatable-cookie/longhorn/pull/36)
at `44b7e3d9` (worker head `181999b3`). Queue task
`258d3724-5d2d-4682-97d7-3a4f08e22ec5` closed at `b43c8a9e`.

## What landed

- `currentSelectionOrigin()` in `@inflatable-cookie/longhorn/agent-control`
  reads the page shim's origin for the consumer to pass into its own command.
  No shim, `"human"`, or any malformed token is human.
- `begin_host_selection` in `longhorn-tauri-agent-control`, under
  `agent-control`. Agent origin publishes a pending selection bound to the
  invoking window and webview and settles with the JS route's result shape.
  Human, missing, or malformed origin returns `UsePlugin` and publishes
  nothing. No native fallback on the agent arm; Longhorn never writes the
  chosen path. The JS command now shares `settle_selection`.
- Consumer commands take `origin: Option<SelectionOrigin>` and call
  `unwrap_or_default()`. A required `SelectionOrigin` would reject an invoke
  without the key, so a human would never reach the dialog.
- Guide, crate README, and skill show the `blocking_save_file` before/after.

## Evidence

- `host_selection` fixtures (9): agent → pending → answer; human, absent,
  malformed → `UsePlugin` with generation unchanged; reject → `null`; expiry,
  shutdown, duplicate, wrong-instance, malformed save fail typed and settle
  once; a real `#[tauri::command]` invoked with `{}` through
  `tauri::test::get_ipc_response` reaches `UsePlugin` with no invoke error.
- TS origin fixtures: no shim, agent after agent input, human after a trusted
  event.
- `effigy check:agent-control-release-absence`: `begin_host_selection`
  present with the feature, absent without.
- `effigy qa` green on the worker head. Independent review: changes required
  on `bfa32f1b` (the missing-key wire path), ready to merge on `181999b3`.

## Limits

- Soundcheck adoption and its live MCP proof are consumer-owned and pending.
  Adoption turns each picker command `async` with `Webview` and
  `State<SelectionRegistry>` parameters; dialog options and result handling
  do not change.
- The wrong-instance fixture uses a separate registry, as the JS route's does.
- The release-absence script names `begin_host_selection` as a literal, not
  through `HOST_SELECTION_ENTRY`.
- Unreleased. Ships with the next Longhorn version.
