# 022 Agent App Control

Status: active
Owner: Longhorn maintainers
Created: 2026-08-19
Updated: 2026-08-20 — child-webview semantic targeting admitted, opt-in
per label at mount, default closed (operator decision; g02.035 executes).
Prior: g02.034 evidence closeout (Card 238): `screenshot`
composes the whole window across child webviews; the tool-surface claim,
native-surface boundary, required evidence, and narrowings updated to the
proved mechanism
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
- Event push uses the `subscriptions/listen` request-scoped SSE stream.
  Console output, page errors, and navigation events are MCP resources
  (`longhorn://agent-control/{console,error,navigation}`); subscribers opt
  in by URI and receive `notifications/resources/updated`. rmcp 3.1.3's
  listen sink rejects custom notifications and logging, so those events
  do not ride as first-class MCP notification methods. Request-scoped
  notifications (progress) stay on their own request stream. The resource
  body carries the bounded event ring and the drop counter.
- Closing a request's response stream cancels that request.

### Availability And Security

- The server exists only behind a dev feature flag. Release builds contain
  none of it. No runtime toggle can enable it in a release build.
- Binds 127.0.0.1 only. Requires a per-instance bearer token. Validates
  `Origin`: a present `Origin` must be a loopback origin, everything else
  is rejected before dispatch. This is the DNS-rebinding defense and is
  not optional — a rebinding attack still presents the attacker's origin.
  Loopback browser origins are admitted (they still need the token, which
  no browser can read) so localhost tooling such as MCP Inspector works;
  Cards 228-229 implement and fixture this rule.
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
- Semantic and input tools target the window's UI webview by default. An
  application may opt in named child-webview labels at mount; an opted-in
  child is a full semantic target addressed by an explicit webview
  parameter, and its refs are scoped to the webview that stamped them —
  a ref never resolves against a webview it did not come from. A child
  not opted in answers typed `Unsupported` naming the absence. The
  default is closed because `evaluate` and synthetic input inside a child
  webview execute in whatever content it hosts; opting in is the
  application asserting that content is its own to drive (operator
  decision, 2026-08-20, from the Figmatic preview-input finding).
- `click`, `type`, `press`, `scroll`, `drag` by ref: synthetic DOM events
  dispatched in-page. They never move the OS pointer and never require
  focus. Documented honestly as untrusted events: native hover, OS
  drag-and-drop, and `isTrusted` checks are out of scope.
- `evaluate`: run JS in the page. Escape hatch, not the primary path.
- `wait_for`: predicate over the semantic tree or page state, bounded by
  timeout. Waiting is DOM-relative, never time- or animation-relative:
  WKWebView coalesces DOM timers in every window state and stops
  `requestAnimationFrame` entirely while the window is not key (Card 227),
  so rAF-driven visuals must not be awaited and elapsed time proves
  nothing about page progress.
- `screenshot`: one image of the whole logical window, composed on macOS
  from every hosted webview's own fresh viewport snapshot (`takeSnapshot`
  reaches only the webview it is called on, so each surface is captured
  separately and drawn at its tauri-reported physical bounds, back to front
  in the view hierarchy's z-order, clipped to the window). Child webviews
  attached to the window appear in the image; hidden ones do not, matching
  what the window shows. Works occluded, unfocused, and minimized — Card
  227 proved the UI webview fresh in all three states and Card 238
  re-proved it per child webview. Requires no screen-recording permission
  or private API. If any hosted visible webview's snapshot fails, the call
  fails typed rather than returning an image that silently omits a surface.
- `command`: invoke a registered contract-006 command by id. This is the
  route to behavior behind native menus and dialogs; agents do not click
  native chrome. An application that composes no command registry mounts
  the provided no-command bridge, and every invocation answers typed
  `Unsupported`; bridging unauthorized invoke surface into `command` is
  not admitted (Figmatic adoption finding, 2026-08-19).
- Window operations: list windows, resize, per-window targeting.

### Boundaries

- Native menus, native dialogs, and OS-level input are out of scope.
  Dev builds may register mock dialog responders; that seam is app-owned.
- Non-webview content (GPUI, native-content islands) is visible in
  screenshots only, never a semantic target. A child *webview* island is
  semantic only when its label is opted in at mount (see Tool Surface);
  otherwise it is screenshot-only like everything else here. On the
  Tauri host, a
  native-content island realized as a child webview attached to the window
  is composed into `screenshot` as described above; a genuinely native
  (non-webview) surface does not appear in the image — the core crate
  exposes a provider seam so such a surface can later register its own
  snapshot and action handlers, and no provider ships under this contract.
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

Satisfied, with the proof named:

- release-build artifact scan proving the server, routes, token code, and
  the injected shim asset are absent — `effigy check:agent-control-release-absence`
  (Cards 230-233)
- stateless conformance: tool calls with no session id succeed; minted
  session ids never appear in responses — Card 229 conformance fixtures
- ref stability fixtures: snapshot, mutate DOM, re-resolve; stale ref fails
  explicitly — Card 232 shim fixtures and Card 233 marshalling
- unfocused-and-occluded screenshot proof on packaged macOS — Card 231
  freshness matrix (`examples/agent-control-proof/evidence/`)
- whole-window screenshot composition across child webviews: both-surfaces
  baseline failure, then every window state fresh for parent and island,
  pixel-exact bounds, clipping, overlap order, and hidden-island absence on
  packaged macOS — Card 238 freshness matrix v2
  (`examples/agent-control-proof/evidence/`)
- Origin-rejection and bad-token fixtures — Cards 229-230
- discovery lifecycle: create, enumerate, stale-pid detection, cleanup —
  Cards 229-231
- two concurrent clients interleaving on one instance without interference —
  Card 229 loopback journal plus Card 234 packaged two-client snapshot and
  listen streams
- one packaged consumer app driven end-to-end by an MCP client while the
  app never holds OS focus — Card 234 `examples/agent-control-proof/e2e.ts`
- opted-in child webview driven end-to-end (snapshot, click, type, drag,
  wait_for, evaluate) unfocused, plus closed-child `Unsupported`,
  cross-webview `UnresolvedRef`, and two-client UI/island interleave —
  Card 240 `examples/agent-control-proof/e2e.ts` (schema v2)

Narrowed, explicitly:

- Capture, `evaluate`, and the semantic tools are proved on macOS only.
  Other hosts compile and answer typed `Unsupported` (contract 020).
- Screenshot composition is proved at 2x backing scale; no 1x display was
  available to probe. The composition works in physical pixels throughout
  (tauri reports child bounds physically; snapshots are drawn into a
  physical-pixel canvas), so scale enters only through the host's own
  physical reporting.
- Genuinely native (non-webview) surfaces do not appear in the composed
  screenshot; no native-surface provider ships, and native chrome is not
  clicked.
- Another-Space window state was not probed (Card 231).
- Listen delivers page events as `resources/updated` on the three URIs
  above, not as custom MCP notification methods.

## Stop Conditions

Stop if an agent needs trusted OS-level input, native-chrome interaction,
release-build availability, remote (non-localhost) access, or driving a
native surface beyond screenshots. Each moves the security or host boundary
and needs its own contract or a provider under a revised one.
