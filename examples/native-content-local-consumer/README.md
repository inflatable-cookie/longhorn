# Native Content Local Consumer

A disposable native consumer proving that Longhorn's public child-view API
hosts a caller-selected bundled local document offline (contract 017). It
carries no product value: every island, window, label and command name is its
own. It is a template, not a root workspace member —
`scripts/verify-native-content-local.ts` copies it into a disposable private
workspace with staged crate sources and builds it there.

## What it proves

One predeclared host window carries one child webview whose initial document
is the bundled local asset `offline/document.html`, attached through the
public `ChildViewSpec` + `ChildViewSource::bundled` API and the ordinary
coordinator plan lifecycle:

- real macOS WKWebView loads the bundled document; the observed child URL is
  the application asset protocol with the bundled document's path;
- the injected initialization script nonce round-trips through the one
  permitted IPC command, proving scoped IPC inside the offline document;
- a second command that exists but is not permitted is denied by the
  capability scope;
- the document's popup and navigation escape probes are denied closed, and
  the adapter reports the escape navigation as a typed `NavigationDenied`;
- a remote fetch triggers a connect-src CSP violation; no page-load event
  leaves the application asset protocol;
- a generated blob download triggers the native download denial;
- detach through the coordinator plan, post-detach observation, and the
  idempotent teardown receipt are truthful.

The host configures an explicit CSP (`default-src 'self'`, IPC connect
sources) and the child's capability grants exactly one command.

Run it through the selectors (never by hand from a workspace that would
resolve the live Longhorn crates):

```sh
effigy check:native-content:local   # staged consumer compiles (check + fmt)
effigy test:native-content:local    # crate-level typed source and lifecycle tests
effigy proof:native-content:local   # the real native run; macOS with a GUI session
```

## Limits

- macOS only: child-view Windows and Linux remain unproved per contract 017.
- Needs a GUI session for the WKWebView; otherwise the proof fails loudly.
- The concrete app-scheme spelling observed here (`tauri://localhost`) is
  Tauri's macOS mapping, recorded as evidence, not a Longhorn promise.
