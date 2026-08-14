# 205 Session Teardown And Entropy

Status: complete
Completed: 2026-08-14
Owner: Tom
Roadmap: g02.022 batch 2
Governing refs: contract 010; contract 007; memo 023 (M-sessions, L-entropy,
L-size)
Depends on: none
Auto-start next card: no

## Objective

Bridge sessions end when their window does, session-id entropy is a documented
requirement, and webview-reachable payloads meet a bound before allocation.

## Why this exists

`BridgeHandlerAssembly` has no close path: sessions are replaced only when the
same caller re-hellos (`crates/longhorn-tauri-bridge/src/handler.rs:99-107`).
A destroyed window's session stays valid indefinitely, and the map grows with
each distinct caller label — consumers creating uniquely-labelled windows grow
it without bound. Session hijack resistance reduces to (window label, session
id) secrecy, and the id is consumer-supplied via `BridgeAuthorityProvider`
with no documented unguessability requirement (fixtures use
`session:fixture-...`; the transfer domain mandates 128-bit entropy at
`docs/architecture/system-architecture.md:476`, the bridge has no equivalent).
And limits are enforced post-allocation: the 256-domain cap is checked in
`TryFrom` after the full `Vec` is deserialized
(`crates/longhorn-bridge/src/negotiation/hello.rs:76-83`); command/query
payloads are unbounded `serde_json::Value`.

## Scope

- `crates/longhorn-tauri-bridge` — teardown wiring
- `crates/longhorn-bridge` — pre-parse discipline where feasible
- contract 010 / architecture doc — the entropy requirement
- `packages/longhorn` bridge client, if teardown has a client half

## Steps

1. Add a session teardown API and wire it to window destroy
   (`on_window_event` or the host's equivalent). A destroyed window's session
   stops validating; test it.
2. Bound the session map: stale-entry eviction or a documented cap, so
   distinct-label growth is not unbounded.
3. Document the unguessable-session-id requirement where the transfer
   domain's rule lives; reference it from contract 010. Consider a host-side
   id helper so consumers do not hand-roll.
4. Byte caps before serde allocation on webview-reachable mutation commands,
   or a recorded reliance on Tauri's transport ceiling — either way, written
   down. Same for `CommandKeymapPatch` collection sizes, which today are
   bounded only after full deserialization.

## Do Not

- Break re-hello replacement — a window renegotiating its session is the
  normal path; teardown is for windows that go away.
- Push entropy generation into fixtures. Predictable ids in tests are fine;
  the requirement binds producers, not fixtures.

## Result

Teardown landed as `BridgeHandlerAssembly::teardown(caller)` —
caller-scoped, idempotent, lock-poison-tolerant — surfaced through
`BridgeCommandService` and `TauriBridgeState::teardown_window`, with the
window-destroyed wiring shown in the doc. The test proves the lifecycle:
query works, teardown, the old session refuses as `InvalidSession` without
dispatching, a second teardown is fine, a fresh hello negotiates clean.
Distinct-label growth is bounded by the same mechanism — a label's sessions
end when its window does.

The entropy requirement is written into contract 010's new Session Lifecycle
section: providers mint unguessable ids, because an id plus a predictable
window label is all session use takes.

The size-discipline decision is recorded rather than implemented: Tauri's
transport deserializes before Longhorn sees bytes, so pre-parse caps are not
reachable at this layer — the reliance is stated, and the rule that binds
Longhorn is bounds-before-work, which the audited limits (hello domain cap,
keymap directive ceiling) already satisfy.

## Acceptance Criteria

- [x] a destroyed window's session fails authorization, with a test
- [x] the session map cannot grow without bound — teardown bounds it per
  label lifetime
- [x] the entropy requirement is written where a consumer will read it
- [x] the pre-parse discipline decision is recorded — reliance on Tauri's
  ceiling, bounds-before-work as the Longhorn rule

## Evidence Required

- teardown wiring and its test
- the documented requirement
- `effigy qa` green

## Stop Conditions

Stop if window-destroy wiring needs lifecycle events the Tauri host does not
expose at the right time — the shutdown-flush precedent (Card 176's finding)
suggests checking before assuming.
