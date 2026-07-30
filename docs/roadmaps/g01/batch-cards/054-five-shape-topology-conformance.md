# 054 Five-shape Topology Conformance

Status: complete
Owner: Tom
Roadmap: g01.009 batch 3
Governing refs: contracts 001, 007, 010, and 012; research memo 013
Depends on: Card 053
Auto-start next card: no
Completed: 2026-07-30

## Objective

Prove the bridge and topology boundary against five read-only donor-derived
fixtures without consumer writes, production networking, or product authority
leakage.

## Scope

- Bovine query-only Tauri-local shape
- Jetstream listener-first whole-snapshot shape
- Soundcheck correlated progress/cancel/terminal and optional-service shape
- Nucleus embedded/optional-host, capability, execution, and per-domain
  authority shape
- Loophole embedded/remote-attach stable-authority shape
- direct, Tauri, and serialized-loopback semantic traces
- no-service and no-event compile/dependency proofs
- topology, capability, authority, retry, and payload audits

## Public Behavior

Every fixture uses the same bridge session and semantic metadata while retaining
its own domain payload types. Optional features disappear from compositions
that do not declare them.

Topology changes never change domain identity or authority meaning. Missing or
failed services remain visible and cannot block unrelated local domains.

## Out Of Scope

- donor repository modification or migration
- production network or packaged service process
- UI reproduction
- product operation implementation
- public package release

## Steps

1. Freeze five consumer-neutral declarations from the promoted memo.
2. Implement the Bovine no-event/no-service trace.
3. Implement the Jetstream listener-first resync trace.
4. Implement the Soundcheck correlated job and optional-service trace.
5. Implement Nucleus embedded, remote, execution-only, and authority-map
   traces.
6. Implement Loophole embedded/remote topology-equivalence trace.
7. Run applicable traces through direct, Tauri, and loopback adapters.
8. Audit dependencies, payloads, capabilities, retries, topology, and
   authority.
9. Record retained, changed, rejected, and deferred donor behavior.

## Acceptance Criteria

- five fixtures contain no donor product payload in Longhorn packages
- Bovine resolves neither event nor service support
- Jetstream receives initial state after listener registration and resyncs gaps
- Soundcheck correlates progress/cancel/terminal and survives optional-service
  absence
- Nucleus separates host connection, capability, execution, and write authority
- Loophole preserves one domain authority across host forms
- all applicable adapter traces produce the same semantic outcomes
- failure in an optional service cannot block local config/window/settings
- dependency and authority audits find no upward optional edge

## Evidence Required

- five fixture declarations and trace outputs
- adapter parity matrix
- no-event/no-service compile graph
- topology and lifecycle matrix
- payload, capability, retry, and authority audits
- behavior delta and deferred-work table

## Stop Conditions

- a fixture requires copying donor product types into Longhorn
- one shape needs a silent authority fallback
- optional service support becomes a foundation dependency
- a production transport is needed to prove semantic conformance
- donor changes are required

## Next Task

Card 055 is ready. Pack the implemented families, install isolated
proofs, publish bridge/topology guidance, run full QA, and close g01.009.

## Result

Completed 2026-07-30.

- Froze five product-neutral topology declarations with exact host, transport,
  capability, authority, import, permission, service, and retry facts.
- Ran query, stream, cancellation, and job semantics through direct, injected
  Tauri, and serialized-loopback adapters.
- Proved Bovine has no event or supervision edge, Jetstream is listener-first
  and gap-safe, and Soundcheck local authority survives service failure.
- Proved Nucleus capability, execution, and write authority remain separate.
- Proved Loophole authority is stable across local-first and remote host
  identity changes.
- Added exact import/capability, payload, credential, optional-edge, retry,
  production-transport, and donor-write audits.
