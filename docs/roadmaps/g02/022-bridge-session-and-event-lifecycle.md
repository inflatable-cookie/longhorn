# g02.022 Bridge Session And Event Lifecycle

Status: ready
Owner: Tom
Updated: 2026-08-14
Governing refs: contract 010; contract 007; memo 023
Depends on: g02.011 (complete)

## Outcome

The bridge's session discipline covers the whole lifecycle: events respect the
read authority the rest of the protocol enforces, sessions end when their
window does, and the example applications a consumer copies start from the
strong posture instead of the weakest.

## Generation Runway

Memo 023 found the bridge boundary well-built per-caller and asymmetric at two
edges: the event channel ignores read authority, and sessions never die. Both
are lifecycle questions, not authorization rewrites — the layered model stays.

## Planning Gaps

- **Event scoping is an unrecorded decision, not an obvious defect.** Contract
  010 calls events "projections and invalidation hints" while the bridge
  publishes full payloads app-wide. Card 204 takes the decision (per-window
  `emit_to` vs hint-only contract) before any code moves. This gap governs
  Card 204's scope the way a decision card governs its own: the card *is* the
  decision, recorded.

## Execution Plan

### Batch 1. Events

- [ ] [Card 204](batch-cards/204-event-scoping-decision.md): decide per-window
  targeting vs hint-only events; implement; add the cross-caller receipt
  negative test either way.

### Batch 2. Sessions

- [ ] [Card 205](batch-cards/205-session-teardown-and-entropy.md): teardown API
  wired to window destroy; unguessable-session-id requirement documented (the
  transfer domain already has one); pre-parse byte caps on mutation-reachable
  commands or a recorded reliance on Tauri's transport ceiling.

### Batch 3. The copy-path

- [ ] [Card 206](batch-cards/206-example-hardening-and-acl-proof.md): the two
  packaged proofs drop `withGlobalTauri`, take a strict CSP, and declare
  Longhorn permission sets; one packaged assertion that a window denied
  `allow-longhorn-bridge-mutate` is actually refused — turning capability
  templates from drift-tested files into behavior-tested policy.

## Dependency Shape

```text
memo 023 (M-events, M-sessions, L-entropy, L-size, L-examples)
 └─ 022 bridge session and event lifecycle
     ├─ 204 event scoping        (decision first)
     ├─ 205 session teardown     (independent)
     └─ 206 example hardening    (independent; adopts 204/205 outcomes if landed)
```

## Goals

- [ ] event delivery and read authority agree, by mechanism or by contract
- [ ] a destroyed window's session stops being valid
- [ ] the strongest posture in the repo is the one examples demonstrate

## Acceptance Criteria

- [ ] the event-scoping decision is recorded in contract 010 with its reason
- [ ] session teardown is wired to a real window-destroy signal and tested
- [ ] a denied window is proved refused a mutate command in a packaged app,
  not only in a mock runtime

## Explicit Non-goals

- Re-opening the authorization model. Per-route checks, strict grammars, and
  epoch discipline held up under audit; this milestone extends them, it does
  not revise them.

## Next Task

Card 204. It is the decision the other two cards' shapes lean on.

## Planning Checkpoint

After Card 204. Per-window targeting touches the event-sink trait both hosts
implement; if the decision goes that way, check the GPUI seam before
proceeding rather than discovering it mid-205.
