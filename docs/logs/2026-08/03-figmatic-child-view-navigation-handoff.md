# Figmatic Child-view Navigation Handoff

Date: 2026-08-03
Roadmap: Figmatic g03.006
Longhorn gate: g01.020 complete

## Resolved Upstream

`ChildViewAdapter::navigate(generation, requested_url)` now changes the
document on one retained child. It checks exact attachment authority, applies
the `ChildViewSpec` navigation policy before native work, avoids reloading the
current URL, and returns `unchanged` or `submitted`. Submission is not load
completion. Page-load events drive readiness for the same generation.

`BackingSurface` remains consumer-rendered native storage, not a second
webview. Figmatic's live generated-Svelte preview remains a `ChildView`.

## Figmatic Resume Gate

Resume g03.006 without a shell-page workaround:

1. keep preview URL construction, local-server origin admission, selection
   meaning, and command authorization in Figmatic;
2. keep the current attach generation beside the preview island state;
3. have the consumer Tauri command parse and normalize the target, then call
   the island's adapter `navigate` operation;
4. retain one child across screen/component selection and let `unchanged`
   suppress reload;
5. keep snapshot-before-hide sequencing and declared visibility inhibitors;
6. test mixed-display scale movement locally because upstream live switching
   remains unproved.

Do not add preview URLs to `@inflatable-cookie/longhorn-native-content` or navigate by global
Tauri label. This handoff resolves the upstream blocker; it does not claim the
Figmatic migration is complete.
