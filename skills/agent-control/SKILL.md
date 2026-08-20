---
name: agent-control
description: Drive a running Longhorn app through its dev MCP control surface instead of OS computer use. Use when testing the running app, driving the UI, taking screenshots of the app, clicking or typing in the app without stealing focus, or finding a local agent-control instance.
longhorn_version: "0.1.0"
---

# Agent Control

For a Longhorn app built with the `dev` feature, this surface is the way
to drive the UI. Do not use OS computer use, screenshots of the desktop,
or synthetic system input against that app. The server exists only in
dev builds, binds `127.0.0.1` only, and treats the per-instance bearer
token as a credential.

## 1. Use this, then confirm the app is running

Use this skill whenever the app under test is a Longhorn consumer (or
the in-repo proof app) with agent-control mounted. Check that an
instance is live before connecting:

```sh
bun skills/agent-control/scripts/find-instance.ts
# after install into a consumer repo:
bun .claude/skills/agent-control/scripts/find-instance.ts
# optional filter:
bun skills/agent-control/scripts/find-instance.ts --app-id com.example.app
```

Exit 0 prints the instance URL and a ready-to-paste `claude mcp add`
line (the token appears only there). Exit nonzero means nothing live —
start the app's **dev** build and rerun; do not fall back to OS input.

## 2. Discovery

Each live instance writes one file. Enumerate the directory; a file
whose pid is dead is leftover from an unclean exit — skip it.

| Platform | Directory |
| --- | --- |
| macOS | `~/Library/Application Support/longhorn/state/agent-control` |
| Linux | `$XDG_STATE_HOME/longhorn/agent-control` (default `~/.local/state/longhorn/agent-control`) |
| Windows | `%LOCALAPPDATA%\longhorn\state\agent-control` |

File name: `<app-id>-<pid>.json`. Schema version 1, camelCase:

```json
{
  "schemaVersion": 1,
  "appId": "com.example.app",
  "pid": 12345,
  "port": 49152,
  "token": "<bearer token>"
}
```

The finder is the one-step form of this. Treat the token like a
password: it is the entire trust boundary (`evaluate` and `command` are
code execution in the app). Never write it into logs, commits, or
diagnostics.

## 3. Connection

The server is **stateless streamable HTTP**. One POST endpoint per
instance: `http://127.0.0.1:<port>/mcp`. It never mints or echoes a
session id. Every call is one self-contained POST. Do not send
`Mcp-Session-Id`.

### Preferred: paste the finder line

```sh
claude mcp add --transport http longhorn-<app-id-sanitized>-<pid> http://127.0.0.1:<port>/mcp --header "Authorization: Bearer <token>"
```

The server name admits only letters, numbers, hyphens, and underscores —
Claude Code rejects dots, so reverse-DNS app ids are sanitized
(`dev.example.app` → `dev-example-app`). The finder's printed line
already is; if you build the name yourself, sanitize it the same way.

Then call the tools through the MCP client.

### Fallback: raw stateless POST

Use this when the client cannot add MCP config. Every request:

```
POST http://127.0.0.1:<port>/mcp
content-type: application/json
accept: application/json, text/event-stream
mcp-protocol-version: 2026-07-28
mcp-method: <method>
authorization: Bearer <token>
```

For `tools/call`, also set `mcp-name: <tool>` and body:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "<tool>",
    "arguments": {},
    "_meta": {
      "io.modelcontextprotocol/protocolVersion": "2026-07-28",
      "io.modelcontextprotocol/clientInfo": { "name": "agent-control", "version": "0.1.0" },
      "io.modelcontextprotocol/clientCapabilities": {}
    }
  }
}
```

The response is SSE. Read the first `data: ` line as JSON. Tool success
is `result.content`; typed failures set `result.isError` and carry a
JSON `ToolError` in the text content.

A present `Origin` header must be a loopback origin or the guard
rejects before dispatch (401/403). Non-browser clients omit `Origin`.

## 4. Tool surface

Refs are stamped into the live DOM at snapshot time and resolved against
the live DOM on use. There is no server-side ref table. `UnresolvedRef`
means the ref is unknown or stale: take a new `snapshot` and use the new
ref. Never retry the same ref blindly.

Input tools dispatch untrusted DOM events in-page. They never move the
OS pointer and never require focus. Native hover, OS drag-and-drop, and
`isTrusted` checks are out of scope — there is no flag that selects a
trusted mode.

`wait_for` is DOM-relative. WKWebView coalesces DOM timers in every
window state and stops `requestAnimationFrame` while the window is not
key, so elapsed time and rAF-driven visuals prove nothing. There is no
time-only or animation-frame wait.

`screenshot` is a fresh image of the whole window: every hosted webview
composed at its bounds, child webviews (native-content islands) included.
It works occluded, unfocused, and minimized. macOS only; other hosts
return `Unsupported`. No screen-recording permission.

After `click`/`type`/`press`/`scroll`/`drag`/`resize_window`, the
receipt means the event was dispatched, not that the UI changed.
Observe with `snapshot` or `wait_for`.

Drive a page in this order: `snapshot` → find a node by `role`/`name` →
act by `elementRef` → `wait_for` a DOM-relative predicate →
`screenshot` to confirm. Use `command` for native chrome. Use
`evaluate` only as an escape hatch.

| Tool | Arguments | Result | Limits |
| --- | --- | --- | --- |
| `click` | `element` (ref), `window?` | `ActionReceipt` | untrusted click; `UnresolvedRef` → re-snapshot |
| `command` | `command` (id), `argument?` | `output?` | contract-006 registry; native menus/dialogs go here, not click. There is no `list_commands` tool — get the id from the operator or the app's composition (the proof worked example registers `proof:ping`). Do not invent ids. Some apps compose no registry at all: every `command` then answers `Unsupported` naming that — drive the UI through snapshot/input and report menu-only gaps to the operator. |
| `drag` | `source` (ref), `target` (ref), `window?` | `ActionReceipt` | untrusted in-page drag; no OS drag-and-drop |
| `evaluate` | `js`, `window?` | JSON `value` | escape hatch; full in-app code execution |
| `list_windows` | _(none)_ | `windows[]` with id, title, size, focused | targeting for `window?` |
| `press` | `key`, `element?`, `modifiers?` (`alt`/`control`/`meta`/`shift`), `window?` | `ActionReceipt` | untrusted key; omit `element` for focused target |
| `resize_window` | `window`, `width`, `height` | `ActionReceipt` | logical pixels; unknown window → `UnknownWindow` |
| `screenshot` | `window?` | PNG image content | whole window incl. child webviews; fresh when occluded/unfocused/minimized; macOS only |
| `scroll` | `delta_x`, `delta_y`, `element?`, `window?` | `ActionReceipt` | omit `element` to scroll the document |
| `snapshot` | `window?` | `window`, `page` (`url`, `title`), `root` tree of `{elementRef, role, name?, value?, states, children}` | refs live-DOM; omit `window` for frontmost |
| `type` | `element` (ref), `text`, `window?` | `ActionReceipt` | untrusted text entry |
| `wait_for` | `predicate`, `timeoutMs`, `window?` | empty result or `WaitTimeout` | see predicates below |

`wait_for` predicates (wire tag `predicate`):

```json
{ "predicate": "refResolve", "element": "<ref>" }
{ "predicate": "refAbsent", "element": "<ref>" }
{ "predicate": "pageUrlContains", "needle": "#about" }
{ "predicate": "pageTitleContains", "needle": "Proof" }
```

Call shape:

```json
{
  "predicate": { "predicate": "pageUrlContains", "needle": "#about" },
  "timeoutMs": 2000
}
```

Typed errors (JSON, `isError`): `UnresolvedRef`, `UnknownWindow`,
`WaitTimeout`, `EvaluationFailed`, `CommandFailed`, `Unsupported`.

### Events

Console, page errors, and same-document navigation are MCP resources,
not custom notification methods. Subscribe with `subscriptions/listen`:

```
mcp-method: subscriptions/listen
```

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "subscriptions/listen",
  "params": {
    "notifications": {
      "resourceSubscriptions": [
        "longhorn://agent-control/console",
        "longhorn://agent-control/error",
        "longhorn://agent-control/navigation"
      ]
    },
    "_meta": {
      "io.modelcontextprotocol/protocolVersion": "2026-07-28",
      "io.modelcontextprotocol/clientInfo": { "name": "agent-control", "version": "0.1.0" },
      "io.modelcontextprotocol/clientCapabilities": {}
    }
  }
}
```

Keep the response stream open. The first SSE event is
`notifications/subscriptions/acknowledged` — that is the subscription
handshake, not a page event. Page events arrive later as
`notifications/resources/updated` with the URI that changed. Then
`resources/read` that URI. Body:

```json
{ "events": [], "nextSeq": 0, "dropped": 0 }
```

`dropped` > 0 means the bounded ring overflowed — events were lost.
Closing the listen response cancels that request.

## 5. Multi-agent etiquette

Two agents may drive two instances, or interleave on one, without
coordination. Refs are shared: a snapshot from agent A is valid for
agent B until the DOM drops the node. Pick the instance by `appId` and
`pid` (the finder accepts `--app-id`). Do not kill another agent's
instance; do not delete a discovery file whose pid is live.

## 6. Stop rules

Do not click native menus or dialogs. Invoke a registered `command`
instead. If the app has no command for that behavior, tell the
operator — do not fall back to OS input.

Do not assume another-Space window state works (unproved). Capture and
semantic tools are macOS-only; `Unsupported` elsewhere is the answer,
not a prompt to screenshot the desktop.

Release builds contain none of this server. If the finder finds
nothing and the app is a release/packaged-without-dev binary, stop and
tell the operator. Never try to enable the surface at runtime.

If a step cannot be done with these tools, tell the operator before
using OS computer use. Falling back silently is the failure this
surface exists to prevent.
