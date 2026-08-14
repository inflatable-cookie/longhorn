# 205 Session Teardown And Entropy

Status: ready
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

## Acceptance Criteria

- [ ] a destroyed window's session fails authorization, with a test
- [ ] the session map cannot grow without bound
- [ ] the entropy requirement is written where a consumer will read it
- [ ] the pre-parse discipline decision is recorded, implemented or reasoned

## Evidence Required

- teardown wiring and its test
- the documented requirement
- `effigy qa` green

## Stop Conditions

Stop if window-destroy wiring needs lifecycle events the Tauri host does not
expose at the right time — the shutdown-flush precedent (Card 176's finding)
suggests checking before assuming.
