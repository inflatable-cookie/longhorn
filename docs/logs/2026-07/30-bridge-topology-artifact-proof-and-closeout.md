# Bridge Topology Artifact Proof And Closeout

Date: 2026-07-30
Card: 055
Roadmap: g01.009

## Result

Five isolated consumers install the bridge TypeScript family from produced
artifacts. Separate query-only and full-host Rust consumers compile offline
from private crate inventories. Full Effigy QA passes. g01.009 is complete.

## Artifact Evidence

TypeScript artifacts:

| Package | Version | SHA-256 |
| --- | --- | --- |
| `@inflatable-cookie/longhorn-core` | 0.1.0 | `b41c3585e48f8e538acaccc68209660e7be55962bdea0af443af924011fcc9f0` |
| `@inflatable-cookie/longhorn-tauri` | 0.1.0 | `4df629c1bc5ebd889bdb29e100956f0a2327fc4fc7cee38843b45a086f7fa3c5` |
| `@inflatable-cookie/longhorn-bridge` | 0.1.0 | `b492c930fe1c2e03d65fce49ec5d928f6b6370e535970451d4b178fa756ba9a8` |

Private Rust inventory archives:

- `longhorn-core` 0.1.0
- `longhorn-bridge` 0.1.0
- `longhorn-tauri-bridge` 0.1.0

Each Rust crate passes `cargo package --list --allow-dirty`. The archives
unpack into a clean temporary workspace and compile offline. They are not
registry-normalized `.crate` files because their interdependent crates remain
private.

## Dependency Matrix

| Shape | Imports | Permissions | Service owner | Query retries |
| --- | --- | --- | --- | ---: |
| Bovine | root, Tauri invoke | query | none | 0 |
| Jetstream | root, stream, Tauri invoke/events | query, listen, unlisten | none | 1 |
| Soundcheck | root, supervision, Tauri invoke/events | query, mutate, listen, unlisten | external local | 2 |
| Nucleus | root, supervision, Tauri invoke | query, mutate | external local | 2 |
| Loophole | root, supervision, Tauri invoke | query, mutate | external remote | 3 |

Every root resolves `@inflatable-cookie/longhorn-bridge`, `@inflatable-cookie/longhorn-core`, and
`@inflatable-cookie/longhorn-tauri` at 0.1.0 plus one `@tauri-apps/api` 2.10.1 peer. No root
contains a workspace alias, sibling package source, or undeclared Longhorn
package.

The query-only Rust graph contains `longhorn-bridge` without supervision or
`longhorn-tauri-bridge`. The full-host graph selects supervision and
`longhorn-tauri-bridge`.

## Semantic Evidence

| Area | Result |
| --- | --- |
| protocol | v1 accepted; v2 rejected as `unsupported_protocol_version` |
| adapters | direct, Tauri, and serialized loopback agree |
| streams | listener precedes snapshot; sequence gap reloads |
| jobs | progress and terminal match request/job; cancellation agrees |
| lifecycle | reconnect invalidates session; renegotiation precedes ready |
| retry | each shape carries an explicit maximum |
| authority | connection, capability, read, write, and execution stay separate |
| service failure | unrelated local domains remain available |

The Loophole trace crosses local-first and remote host forms while preserving
domain authority. Nucleus proves an advertised capability cannot grant denied
write or execution authority. Soundcheck proves optional-service failure does
not remove unrelated local authority.

## Boundary Audits

- generated bridge bindings have zero drift
- packed bridge sources contain no donor product name or fixture domain
- generic Rust/TypeScript sources contain no production networking dependency
- credential input remains an opaque reference; no credential value enters
  artifacts or evidence
- domain payloads, operation names, and product authority remain external
- event imports and Tauri listen/unlisten capabilities agree exactly
- supervision import and declared service ownership agree exactly
- no proof claims deployment, endpoint, discovery, authentication,
  provisioning, update, or remote lifecycle support

## Behavior Delta

| Class | Result |
| --- | --- |
| retained | product domains, payloads, handlers, authority, topology choice, service selection, endpoint and credential policy |
| changed | structural negotiation, correlation, failure classes, ordering, retry gates, reconnect, Tauri assembly, injected supervision |
| rejected | request-id replay permission, renderer fallback authority, load-before-listen, raw product string bus, generic secret values |
| deferred | donor cutover, public release, production networking, discovery, pairing, provisioning, updates, offline mutation queues |
| platform-limited | artifact and mock-host proof; no packaged Windows, Linux, or network runtime claim |

## Validation

- `effigy proof:bridge-topology-artifacts`
- five isolated TypeScript checks and executable semantic traces
- two offline Rust consumer checks
- exact package, import, capability, protocol, lifecycle, retry, credential,
  payload, and authority audits
- full `effigy qa` passed after one source-proof scope repair

## Closeout

Cards 049-055 and g01.009 are complete. Northstar stops at the g01.010 intent
checkpoint. Contract 006 must be revalidated against the completed bridge
before a command-system card is compiled or promoted.
