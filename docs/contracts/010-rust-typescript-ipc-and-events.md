# 010 Rust, TypeScript, IPC, And Events

Status: active compiled boundary
Owner: Tom
Updated: 2026-07-31
Evidence: `../research/translation-memos/003-foundation-boundary-characterization.md`;
`../research/translation-memos/013-typed-bridge-and-backend-topology-boundary.md`;
`../research/translation-memos/014-command-input-and-palette-boundary.md`

## Boundary

Rust owns serialized Longhorn domain contracts. Generated TypeScript types,
thin clients, and testable Tauri handlers expose that authority. IPC is an
adapter seam, not a second product command bus.

The shared bridge owns structural session, topology, correlation, error,
retry, and ordering metadata. Domain packages own operation names, payloads,
validation, snapshots, revisions, event meaning, and write policy.

Command ids remain discovery and input identities under contract 006. A
consumer maps an admitted command to one typed domain operation. Neither the
bridge nor a shared Tauri adapter accepts a generic `{ commandId, args }`
execution payload.

## Type Authority

- Rust `serde` types are authoritative for Longhorn payloads.
- `ts-rs` generates TypeScript into the owning domain package.
- Generated files are checked in for package consumers.
- CI regenerates and fails on drift.
- Consumer-owned product payloads remain consumer-owned and may use their own
  checked generation seam.
- Longhorn clients do not maintain handwritten duplicate DTOs.
- Adapters do not rename serialized keys or enum values after receipt.

## Names And Envelopes

- Tauri commands use `longhorn_<domain>_<verb>`.
- Events use `longhorn://<domain>/<kind>`.
- Requests are typed queries, commands, or cancellation requests.
- Requests carry a request id and may carry an expected revision, client id,
  session id, or distinct idempotency key.
- A request id is correlation. It is not replay permission.
- Mutations return the request id, authoritative revision, and result or
  current snapshot.
- Errors have stable code, message, retry class, failure phase, and optional
  structured details. Bare strings are adapter failures, not protocol errors.
- Snapshots and authoritative events carry an authority epoch plus monotonic
  revision or sequence.
- Progress and terminal events echo their initiating request id.
- Cancellation returns a receipt and does not imply immediate termination.

## Subscription Handshake

Clients:

1. attach the event listener
2. query the current snapshot
3. accept the newest epoch and revision
4. ignore duplicate or older messages
5. resync on a revision gap or epoch change

Teardown is idempotent and safe when an asynchronous listener registration
completes after disposal. Events are projections and invalidation hints, not
durable delivery. Query-only domains do not require an event transport.

## Handler And Client Shape

- One handler assembly function is used by the real Tauri app and mock-runtime
  tests.
- Domain packages own typed command, query, and subscription clients.
- Command catalogue and keymap adapters may expose typed metadata, preview,
  and mutation queries. They expose no generic product command executor.
- Raw `invoke` and `listen` calls for Longhorn domains remain inside the
  bridge package.
- Direct and serialized adapters pass the same domain conformance suite.
- Every command validates current domain authority and capability. Tauri
  capability files are necessary host policy, not sole authorization.
- Host connection, authentication, capability advertisement, and domain
  authority are checked separately.
- Domain clients expose indeterminate outcomes for uncertain non-idempotent
  writes.

## Compatibility

- Additive optional fields are permitted within a compatible protocol line.
- Removing or changing field meaning requires a breaking package version.
- Unknown future enum variants produce an explicit incompatibility result.
- Bridge startup exposes exact protocol version, session/host identity,
  transport features, capability advertisement, and authority descriptors.
- v1 uses exact-version negotiation. Version ranges wait for a real
  rolling-upgrade requirement.

## Boundary Validation Target

Decided 2026-08-10, after measuring what the boundary does today rather than
what it was assumed to do.

**The TypeScript boundary validates to the same strictness as the Rust
authority, and derives that strictness from it.** Concretely:

- **Unknown fields are rejected.** Rust declares `deny_unknown_fields` on 332
  types. Nine of thirteen TypeScript packages accepted them, so a payload
  TypeScript waved through was one Rust would refuse. That asymmetry was
  accidental — it is the shape of who wrote which validator, not a decision
  anyone took.
- **Missing fields are rejected.** `ts-rs` emits `Option<T>` as
  always-present-and-nullable, so there is no absent-versus-null ambiguity to
  navigate: zero optional fields across the generated protocol, 88 explicit
  `| null`. A derived validator can be exactly as strict as a hand-written one
  without guessing.
- **Every bound comes from a named constant, never a literal.** A `MAXIMUM_*`
  that bounds a wire-visible collection is enforced from a generated constant,
  and `check:bindings` fails when Rust changes and TypeScript does not.
- **A rule Rust declares is generated, not copied.** Including rules that are
  not types: the connection state/reason matrix lived in a `matches!` arm, was
  hand-copied into TypeScript, and agreed by maintenance rather than by
  construction until it was declared and emitted.

### What this costs, stated plainly

Nine packages become stricter than they were. A consumer sending a field the
protocol does not declare starts failing where it was previously ignored.
Every consumer is on a `file:` install, so this surfaces during a coordinated
change rather than in the field — which is the argument for doing it now
rather than after publication.

### What it is not

Not forward compatibility. Ignoring unknown fields would let a newer host add
one without breaking an older client, and that is a real posture with real
advantages. It is rejected here because it diverges from the authority, and a
boundary that is deliberately less strict than the thing it defends needs its
own written rule and its own drift gate. Longhorn has one authority; the
boundary derives from it.

If rolling upgrades ever require it, that is a version-negotiation change and
belongs beside the exact-version rule above — not a per-package validator
loosening.

## v1 Adapter Proof

- direct/in-process execution
- Tauri invoke/event execution
- deterministic serialized loopback execution

The loopback proves serialization and semantic parity. It does not claim a
production network protocol. Service transports implement the same injected
port after their production security and platform contract is selected.

The operation domain proves the same checked state through direct,
JSON-serialized, Tauri, and bridge-domain composition. Bridge request and job
ids remain correlation; operation id and authority cursor remain domain
authority.

## Acceptance

- generated TypeScript has a zero-diff regeneration check
- a Tauri mock runtime exercises the same handler assembly as the app
- listener-before-snapshot tests cannot miss an intervening mutation
- duplicate, stale, gap, and new-epoch fixtures behave deterministically
- late listener registration can be disposed without leaking
- progress, cancellation, terminal, and indeterminate-write fixtures remain
  request-correlated
- typed error fixtures round-trip between Rust and TypeScript
- no Longhorn Svelte component calls raw Tauri IPC

## Implemented Evidence

Cards 049-055 generate and zero-diff check the exact-v1 bridge protocol,
execute one semantic contract through direct, Tauri, and serialized-loopback
adapters, and install five isolated consumers from packed TypeScript
artifacts. Import inventories distinguish invoke-only, stream, event, and
supervision edges. Capabilities remain exact outer admission; per-domain
authority remains separately checked.

Private Rust crates pass package inventory checks and offline unpacked
consumer builds. Registry-normalized Cargo artifacts remain a public
release-lane gate.
