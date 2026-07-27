# g01.009 Typed Bridge And Optional Backend Topology

Status: blocked on `g01.001`; can research beside foundation work  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: contract 007; IPC/event contract pending

## Outcome

Use one semantic client contract across direct, Tauri-local, local-service, and
remote adapters.

## Batches

### 1. Wire authority

- choose schema/code generation or checked-binding strategy
- define command/query/event/error envelopes
- initial snapshot, ordering, correlation, cancellation, and listener lifetime

### 2. Adapter conformance

- in-process/direct adapter
- Tauri invoke/event adapter
- one serialized service adapter
- shared semantic fixture suite

### 3. Readiness and authority

- capabilities, versions, connection state, reconnect, and shutdown
- one write authority per domain
- stale-event and duplicate-command protection
- secure credential adapter integration

### 4. Topology examples

- Bovine local-only
- Nucleus-style optional service
- Loophole-style local process boundary

## Acceptance

- direct and serialized adapters produce the same outcomes
- local configuration/windowing works with no server
- version mismatch and reconnect states are actionable
- an offline projection cannot silently become write authority

