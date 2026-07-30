# Reconnect, Retry, And Injected Supervision

Date: 2026-07-30
Card: 053
Roadmap: g01.009

## Result

Added one checked connection lifecycle to Rust and TypeScript. Ready requires
a valid negotiation receipt plus every consumer-declared domain authority.
Each state change returns monotonic previous/current evidence. Reconnect
invalidates the active session and authority map before scheduling bounded
injected backoff. Transport-ready re-entry before the scheduled monotonic
deadline fails without changing state.

Current-session cursor checks distinguish current, superseded-session, stale
authority, future unnegotiated authority, and unknown-domain evidence. A fresh
negotiation resets the reconnect budget and becomes the only current
authority.

## Retry

Query retry combines the existing query retry decision with an explicit
0–64 attempt ceiling, injected monotonic clock, and injected backoff. A
`never` class or exhausted budget goes offline or returns no query schedule.

Command replay rules did not move. An uncertain command reuses the same
request only when it has a separate durable idempotency key and the authority
advertises finite deduplication. Every other uncertain write is
indeterminate. Request correlation alone grants nothing.

## Optional Supervision

Rust supervision is feature-gated. TypeScript supervision is isolated at
`@longhorn/bridge/supervision`; the package root does not import it.

The injected port supports spawn, attach, readiness, restart, reconnect, and
shutdown observations. Only owned local services admit spawn, restart, or
shutdown. External local and remote attachments may reconnect but cannot be
stopped or replaced through Longhorn.

The port receives only an optional opaque `BridgeCredentialRef`. Outcomes are
closed stable codes. Rust accepts no arbitrary failure text; TypeScript
rejects malformed outcomes and converts thrown adapter errors to a generic
redacted error. No config snapshot, receipt, event, error, or diagnostic API
accepts credential material.

## Boundary Audit

- no executable path, discovery, acquisition, installer, updater, or download
- no endpoint discovery, pairing, authentication provider, or credential store
- no HTTP, WebSocket, socket, named-pipe, or async-runtime dependency
- no remote lifecycle ownership
- no offline mutation queue
- no consumer, Tauri, Svelte, Poodle, or domain payload import
- no-service TypeScript consumers resolve no supervision runtime
- no-service Rust consumers leave the `supervision` feature disabled

## Validation

- `effigy test:bridge-core`
- `effigy test:bridge-ts`
- bridge TypeScript check, binding drift check, and dry-run pack
- focused Rust Clippy and formatting
- `git diff --check`

## Next

Card 054 is ready. Compose the completed seam into local-only, embedded,
supervised-local, and remote-attach topology fixtures.
