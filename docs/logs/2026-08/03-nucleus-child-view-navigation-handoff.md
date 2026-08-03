# Nucleus Child-view Navigation Handoff

Date: 2026-08-03
Consumer lane: later native-browser control cleanup
Longhorn gate: g01.020 complete

## Current Bypass

Nucleus `browser_panel_navigate` and `browser_panel_current_url` call
`AppHandle::get_webview(child_label)` and operate on a global Tauri handle.
That bypasses the generation authority already held by `BrowserPanelRuntime`.

## Cleanup Gate

In a separate Nucleus batch:

1. resolve the island under `BrowserPanelRuntime` and read its coordinator's
   current attach generation;
2. retain Nucleus URL normalization and browser authorization policy;
3. call `island.adapter.navigate(generation, url)` and return its exact
   outcome instead of claiming a load completed;
4. call `island.adapter.current_url(generation)` for native observation;
5. retain Nucleus-owned back, forward, reload, history, and browser meaning;
6. prove stale generations and missing islands fail without label lookup.

This is cleanup of an existing consumer bypass. It does not block Figmatic and
does not claim Nucleus browser navigation is migrated.
