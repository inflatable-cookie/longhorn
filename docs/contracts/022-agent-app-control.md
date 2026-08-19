# 022 Agent App Control

Status: draft, not compiled
Owner: Longhorn maintainers
Created: 2026-08-19
Depends on: contracts 001, 006, 010, 012, 020
Affects: new `longhorn-agent-control`, new `longhorn-tauri-agent-control`,
`longhorn` (TS shim), all app consumers in dev builds

## Problem

Agents developing consumer apps test them through OS-level computer use:
screen capture plus synthetic system input. That steals focus and the
pointer, serializes every agent behind one desktop, and makes the machine
unusable during long runs. No standard remote-control protocol reaches the
running app on macOS: WKWebView speaks neither CDP nor WebDriver, and
`tauri-driver` does not support macOS. The apps need a control surface an
agent can use while the app runs unfocused in the background.

## Contract

### Protocol

- The control surface is an MCP server over streamable HTTP, stateless only.
  One POST endpoint per app instance. The server never mints or echoes a
  session id. Each tool call is a self-contained request.
- Semantics follow MCP revision 2026-07-28. The implementation library may
  negotiate earlier revisions with older clients; stateless behavior does
  not change per revision.
- Event push uses the `subscriptions/listen` request-scoped SSE stream:
  console output, page errors, and navigation events. Request-scoped
  notifications (progress) stay on their own request stream.
- Closing a request's response stream cancels that request.

### Availability And Security

- The server exists only behind a dev feature flag. Release builds contain
  none of it. No runtime toggle can enable it in a release build.
- Binds 127.0.0.1 only. Requires a per-instance bearer token. Validates
  `Origin` and rejects browser-originated requests; this is the DNS-rebinding
  defense and is not optional.
- `evaluate` and command invocation are full code execution in the app.
  The token is the entire trust boundary; treat its file like a credential.

### Discovery

- Each instance writes a discovery file at
  `$XDG_STATE_HOME/longhorn/agent-control/<app-id>-<pid>.json`
  (macOS: `~/Library/Application Support` state equivalent) containing app
  id, pid, port, token, and schema version. Removed on clean exit; stale
  files are detectable by dead pid.
- Agents enumerate the directory to find live instances. Two agents may
  drive two instances, or interleave on one, without coordination.

### Tool Surface

- `snapshot`: semantic element tree of the webview (roles, names, values,
  state) with stable element refs. Refs are stamped into the live DOM at
  snapshot time and resolved against the live DOM on use. No server-side
  ref table; a ref from any prior snapshot either resolves or fails
  explicitly.
- `click`, `type`, `press`, `scroll`, `drag` by ref: synthetic DOM events
  dispatched in-page. They never move the OS pointer and never require
  focus. Documented honestly as untrusted events: native hover, OS
  drag-and-drop, and `isTrusted` checks are out of scope.
- `evaluate`: run JS in the page. Escape hatch, not the primary path.
- `wait_for`: predicate over the semantic tree or page state, bounded by
  timeout.
- `screenshot`: window image via webview snapshot capture. Works occluded
  and unfocused; requires no screen-recording permission.
- `command`: invoke a registered contract-006 command by id. This is the
  route to behavior behind native menus and dialogs; agents do not click
  native chrome.
- Window operations: list windows, resize, per-window targeting.

### Boundaries

- Native menus, native dialogs, and OS-level input are out of scope.
  Dev builds may register mock dialog responders; that seam is app-owned.
- Non-webview content (GPUI, native-content islands) is visible in
  screenshots only. The core crate exposes a provider seam so a native
  surface can later register its own snapshot and action handlers; no
  provider ships under this contract.
- The control server is observation and input only. It holds no app state,
  no history, no authority. App semantics stay behind existing commands and
  IPC per contracts 006 and 010.
- Package split follows the existing pattern: `longhorn-agent-control`
  host-agnostic (tools, discovery, token, provider seam),
  `longhorn-tauri-agent-control` the Tauri host wiring, TS shim in
  `longhorn` for snapshot/dispatch inside the webview. A GPUI host
  composes the core crate and its own provider or nothing; absence is not
  a gap.

## Compatibility And Migration

Additive. No consumer changes until an app opts in by enabling the dev
feature and mounting the plugin. Discovery file schema carries a version
field from day one; pre-1.0, schema breaks bump it without compatibility
reads.

## Required Evidence

- release-build artifact scan proving the server, routes, and token code
  are absent
- stateless conformance: tool calls with no session id succeed; minted
  session ids never appear in responses
- ref stability fixtures: snapshot, mutate DOM, re-resolve; stale ref fails
  explicitly
- unfocused-and-occluded screenshot proof on packaged macOS
- Origin-rejection and bad-token fixtures
- discovery lifecycle: create, enumerate, stale-pid detection, cleanup
- two concurrent clients interleaving on one instance without interference
- one packaged consumer app driven end-to-end by an MCP client while the
  app never holds OS focus

## Stop Conditions

Stop if an agent needs trusted OS-level input, native-chrome interaction,
release-build availability, remote (non-localhost) access, or driving a
native surface beyond screenshots. Each moves the security or host boundary
and needs its own contract or a provider under a revised one.
