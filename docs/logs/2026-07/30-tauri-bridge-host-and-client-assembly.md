# Tauri Bridge Host And Client Assembly

Date: 2026-07-30
Card: 052
Roadmap: g01.009

## Result

Added `longhorn-tauri-bridge` as the narrow registered-domain host edge for
the generic bridge protocol. One assembly handles direct calls and mock/real
Tauri state. It exposes hello, authority refresh, query, command,
cancellation, and resync through stable `longhorn_bridge_*` commands.

Typed domain handlers are erased only inside the registry. The host checks
caller ownership, current session, route/domain pairing, advertised
capability, read/write/execution authority, and command epoch before invoking
them. Reply, snapshot, cancellation, progress, and terminal correlation is
checked at publication or return.

## Optional Events

Query-only assembly has no event sink and rejects publication. Subscription
assembly accepts an injected real Tauri or mock sink and publishes only:

- `longhorn://bridge/domain`
- `longhorn://bridge/progress`
- `longhorn://bridge/terminal`

The TypeScript raw Tauri root is now invoke-only. Event support moved to
`@longhorn/tauri/events`. `@longhorn/bridge/tauri` composes checked sessions
and generic operations over invoke. `/tauri-events` adds listener-first
snapshot resync plus codec-checked, request/job-correlated progress and
terminal listeners.

## Capability Audit

The query-only example admits hello, authority, query, and resync with no
event permission. The subscription example adds command, cancellation, and
event listen/unlisten. These files govern platform reachability only. They
cannot create sessions or domain authority.

No Rust or TypeScript adapter imports a consumer, Svelte, or Poodle package.
Routes and payload vocabulary remain injected by domain adapters.

## Evidence

- invalid session, route/domain, and write authority reach no handler
- typed query and cancellation handlers preserve correlation
- direct and Tauri mock hello/query traces match
- query-only publication fails explicitly
- injected event publication carries the stable checked envelope
- Tauri stream composition listens before resync
- job listeners reject wrong correlation, duplicates, and post-terminal data
- query-only Tauri import never loads event support
- Card 052 introduces no god-file high finding

## Validation

- `effigy test:tauri-bridge`: 6 passed
- `effigy test:bridge-ts`: 14 passed, 93 expectations
- `effigy test:tauri-ts`: 4 passed, 18 expectations
- TypeScript checks and bridge/Tauri dry-run packs passed
- focused Rust Clippy and formatting passed
- `git diff --check`

## Next

Card 053 is ready. Implement deterministic reconnect, retry, authority
invalidation, and injected supervision without selecting a production
transport.
