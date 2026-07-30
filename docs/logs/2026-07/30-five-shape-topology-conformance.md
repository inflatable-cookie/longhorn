# Five-shape Topology Conformance

Date: 2026-07-30  
Card: 054  
Roadmap: g01.009

## Outcome

Five product-neutral declarations now prove the bridge topology seam before
artifact packaging. They use injected direct, Tauri-shaped, and serialized
loopback adapters. No proof opens a production connection or writes a donor
repository.

The declarations fix host forms, transport features, domains, capability,
authority, imports, Tauri permissions, service ownership, and bounded query
retry. Product payloads stay outside shared packages.

## Trace Matrix

| Shape | Trace | Result |
| --- | --- | --- |
| Bovine | query through direct, Tauri, loopback | equal reply; no event or service edge |
| Jetstream | listen, snapshot 0, gap, snapshot 2 | equal ordering and final state on all adapters |
| Soundcheck | progress, wrong correlation, terminal, late messages | equal five-decision trace; cancellation parity; local domains stay live |
| Nucleus | direct/local-service, query, execution, denied write | connection, capability, execution, write remain separate |
| Loophole | local-first/remote query and authority | host/session change; domain authority stays exact |

Jetstream records `listen → snapshot → snapshot`; the second snapshot follows
the sequence gap. Soundcheck records `accept → ignoreWrongCorrelation →
accept → ignoreAfterTerminal → ignoreAlreadyTerminal`.

## Dependency And Topology Matrix

| Shape | Event subpath | Supervision subpath | Host forms | Service ownership |
| --- | --- | --- | --- | --- |
| Bovine | absent | absent | Tauri local | none |
| Jetstream | present | absent | direct, Tauri local | none |
| Soundcheck | present | present | Tauri local, local service | external local |
| Nucleus | absent | present | direct, local service | external local |
| Loophole | absent | present | local first, remote | external remote |

The verifier walks each fixture's actual local import closure and compares it
with the declaration. Event imports must match event permissions. Supervision
imports must match explicit ownership. The bridge root barrel imports none of
the stream, Tauri-event, or supervision subpaths. Rust supervision remains an
opt-in feature.

## Authority And Failure Evidence

- Bovine admits only the generic query capability.
- Jetstream registers before snapshot load and resyncs a gap.
- Soundcheck marks the service domain offline while local config, window, and
  settings domains stay authoritative and writable. Failed service attachment
  does not block their local graph or query path.
- Nucleus advertises mutation and cancellation capabilities on domains that
  lack the matching write or execution authority. Both fail closed.
- Loophole changes host instance, session, authentication posture, and host
  form without changing domain scope or authority facts.
- Retry limits are fixture-owned and exhaust to no schedule: 0, 1, 2, 2, and
  3 attempts across the five shapes.

## Payload And Boundary Audit

Shared Rust and TypeScript bridge source contains none of the five donor
names. Fixture domain and payload vocabulary uses the `fixture.*` namespace
and one numeric probe. The proof contains no credential vocabulary, raw Tauri
API import, network endpoint, socket, WebSocket, or fetch call.

The adapters prove semantics only. Production transport, endpoint discovery,
pairing, authentication providers, service acquisition, installation, and
updates remain injected integration work.

## Donor Behavior Decisions

| Decision | Behavior |
| --- | --- |
| Retained | Bovine query-only admission; Jetstream whole-snapshot resync; Soundcheck correlation; Nucleus per-domain authority; Loophole stable scope |
| Changed | Donor operations and payloads become neutral probe fixtures; topology identity is explicit protocol data |
| Rejected | capability-as-authority, silent service fallback, implicit retry, event permission without event import |
| Deferred | production service transport, security, discovery, provisioning, update delivery, donor migration |

## Evidence

- `examples/bridge-topology-proof/declarations.json`
- `examples/bridge-topology-proof/proof.ts`
- `scripts/verify-bridge-topology-conformance.ts`
- Rust donor-shape negotiation tests in
  `crates/longhorn-bridge/tests/bridge_contract/donors.rs`

Validation:

```text
effigy proof:bridge-topology
effigy test:bridge-ts
effigy test:bridge-core
effigy test:tauri-bridge
cargo clippy -p longhorn-bridge -p longhorn-tauri-bridge --all-targets --features longhorn-bridge/supervision -- -D warnings
cargo fmt --all --check
effigy qa:docs
effigy qa:northstar
```

All pass. The proof reports direct/Tauri/loopback parity, exact import graphs,
matching capability graphs, no upward optional edge, no production transport,
and no donor writes.

## Next

Card 055 is ready. Pack the Rust and TypeScript families, install five
isolated proof roots, publish composition and migration guidance, run full
Effigy QA, and close g01.009.
