# Nucleus Native Browser Cutover

Date: 2026-08-01
Status: complete
Card: 100

## Outcome

Nucleus commit `ce71af24c7f042c16d0f44ee4f13332cb4fdfa98`
replaces its renderer-created Tauri Webview path with the Longhorn
native-content host, child-view adapter, checked client, and Svelte session.
Longhorn commit `920bb8c1f19e0ce3a6a5766598d2bfa488ddde63`
adds the bounded native policy hook required to retain Nucleus trusted chrome
without adding browser payloads to the shared protocol.

Each Browser panel maps to one stable
`island:nucleus-browser:{panel-id}`. The host retains the last attach
generation, advances it after destroy/replacement, applies exact plans through
`TauriChildViewRuntime`, admits fresh observations, and emits protocol changes
only to the bundled main controller. Panel close destroys. Ordinary tab or
project unmount hides the child for process-lifetime reuse. App exit drains the
registry and tears down every remaining child.

## Renderer And Geometry

`BrowserPanel.svelte` owns one `NativeContentSession` per mount and binds it to
the exact viewport element. It supplies device scale, nearest rounding through
host state, native-direct input, and explicit visibility inhibitors for
unmount, inactive panel, empty viewport, workspace gesture, and intersecting
overlay. Listener installation precedes session start.

The old renderer `Webview` import, label lookup, ensure, set-bounds, show, hide,
and close path is gone. The Tauri capability no longer grants raw Webview
show/hide/close permissions. Its sole controller selector remains `main`; the
remote child matches no capability.

## Retained Nucleus Policy

Nucleus still owns:

- initial source and HTTP/HTTPS admission, including HTTPS defaulting
- back, forward, reload, URL entry, and explicit system-browser open
- popup and download denial with trusted-chrome notices
- normal shared engine data-store selection through an explicit `None`
- the rule that Nucleus persists no cookies or credentials
- the allowlisted macOS cursor bridge and cursor reset
- Browser panel identity, close, unmount, and app-host lifecycle choices

Longhorn owns child construction, bounds, show/hide, generation checks,
observation, execution receipts, and teardown. The optional initialization
script is bounded to 64 KiB, rejects NUL, and remains native-only. Page-load,
denied-popup, denied-download, and macOS document-title events return through
the native observer, not the renderer protocol.

## Packaged macOS Evidence

An isolated release bundle with the canonical app identifier rendered Example
Domain in the native child. The first visible plan was attach, physical bounds,
show, and native-direct input. Switching to Agent Chat applied bounds then
`Hide(nucleus:unmounted)`. Returning to Browser applied bounds then show on
attach generation 1 with no second attach. Overlay precedence, URL
normalization/rejection, window resize geometry, close/replacement, and cursor
reset also passed in native development smoke.

The temporary display-name override existed only to target the proof process
while unrelated Nucleus processes were open. Product identity remained
`com.inflatablecookie.nucleus`.

## Validation

- Longhorn child-view unit tests: 1 passed
- Longhorn child-view adapter tests: 8 passed
- Longhorn child-view check, graph audit, Clippy, and proof build: pass
- Nucleus Browser policy tests: 4 passed
- Nucleus panel guards: 10 passed
- Nucleus renderer tests: 39 passed
- Nucleus mounted layout tests: 2 passed
- Nucleus Rust check and production renderer build: pass
- Nucleus Svelte check: 0 errors; one pre-existing ProjectRail ARIA warning
- frozen Bun install: pass
- `effigy proof:nucleus-native-browser-cutover`: pass

The dependency graph uses private sibling `file:` sources with exact
overrides. No Surface package or duplicate Svelte runtime enters Nucleus.
Package-manager publication remains deferred.

## Next

Card 101 runs final restart, rollback, artifact, duplicate-code, capability,
and no-Surface conformance, then closes the Nucleus migration milestone.
