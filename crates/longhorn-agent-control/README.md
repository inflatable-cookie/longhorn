# longhorn-agent-control

Host-agnostic core of the agent app-control surface
(`docs/contracts/022-agent-app-control.md`):

- tool vocabulary as types — requests, results, and errors for `snapshot`,
  `click`, `type`, `press`, `scroll`, `drag`, `evaluate`, `wait_for`,
  `screenshot`, `command`, and window operations
- discovery-file lifecycle at `<state root>/longhorn/agent-control/`
  (path resolution through the contract 004 storage-profile conventions)
- per-instance bearer token: CSPRNG generation, constant-time verify,
  redacted `Debug`
- the stateless MCP streamable-HTTP server assembly over rmcp
  (`legacy_session_mode: false`; bearer token and `Origin` rejection run
  before tool dispatch), as an axum router a host mounts
- page events as MCP resources (`longhorn://agent-control/{console,error,navigation}`)
  over `subscriptions/listen` — rmcp's listen sink carries
  `resources/updated`, not custom notifications
- the native-surface provider seam; no provider ships under contract 022

Synthetic `click`/`type`/`press`/`scroll`/`drag` are untrusted DOM events:
they never move the OS pointer and never require focus. Native hover, OS
drag-and-drop, and `isTrusted` checks are out of scope. `wait_for` admits
only DOM-relative predicates.

No host dependency: no tauri, wry, or objc2. The Tauri host (g02.031)
implements `ControlHandler` and mounts the router; a GPUI host composes the
core and its own provider or nothing — absence is not a gap.
