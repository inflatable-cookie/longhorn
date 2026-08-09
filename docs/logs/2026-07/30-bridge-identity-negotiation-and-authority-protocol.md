# Bridge Identity, Negotiation, And Authority Protocol

Date: 2026-07-30
Card: 049
Roadmap: g01.009

## Result

Added `longhorn-bridge`, a pure Rust exact-v1 negotiation and authority
protocol. The crate depends only on `longhorn-core` and Serde. It owns no
product payload, transport, process, renderer, or consumer lifecycle.

## Protocol

- bounded bridge, session, host-instance, domain, capability, authority-scope,
  transport-feature, and diagnostic identities
- exact v1 hello request and negotiated receipt
- direct, Tauri-local, local-service, remote, and local-first host forms
- checked connection state/reason pairs and explicit retry posture
- authentication posture independent of connection and domain authority
- transport features independent of domain capabilities
- capabilities independent of read, write, and execution authority
- nonzero authority epoch plus optional authoritative revision evidence
- one current writer maximum per authority scope
- absent domains omitted; unrequested domains rejected by receipt validation

All validated public descriptors keep fields private and route deserialization
through checked constructors. Fixed collection and message ceilings prevent
unbounded wire input.

## Fixture Matrix

| Shape | Host posture | Proof |
| --- | --- | --- |
| Split-shell | Tauri-local, request/reply | query only; no subscription or service feature |
| Nucleus | direct and remote | host/session identities differ; execution ownership grants no write authority |
| Loophole | local-first and local-service | host form changes while domain and authority-scope identity remain stable |

Additional fixtures cover incompatible versions, duplicate domains and
features, invalid connection reasons, offline authority, multiple writers,
absent and unrequested domains, serialization, and collection bounds.

## Boundary Audit

- `longhorn-bridge` dependencies: `longhorn-core`, `serde`
- no Tauri, Tokio, async trait, HTTP, socket, renderer, service manager, or
  donor dependency
- capability does not imply authority
- connection does not imply authentication or domain access
- execution ownership does not imply write authority
- host-instance and session identities are distinct Rust types

## Validation

- `cargo test -p longhorn-core -p longhorn-bridge`
- 12 bridge contract tests
- `cargo clippy -p longhorn-core -p longhorn-bridge --all-targets -- -D warnings`
- `effigy qa`

## Next

Card 050 is ready. Add typed operation, failure, retry, stream, and job metadata
without product payload or transport authority.
