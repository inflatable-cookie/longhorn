# g02.022 Bridge Session And Event Lifecycle

Status: complete
Completed: 2026-08-14
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

- ~~**Event scoping is an unrecorded decision, not an obvious defect.**~~
  Closed by Card 204: per-window targeted delivery, because the client
  already dropped foreign-session cursors — broadcast was delivery without a
  consumer.

## Execution Plan

### Batch 1. Events

- [x] [Card 204](batch-cards/204-event-scoping-decision.md): decide per-window
  targeting vs hint-only events; implement; add the cross-caller receipt
  negative test either way. **Landed 2026-08-14** — targeted delivery: the
  client already dropped foreign sessions, so broadcast had no consumer;
  `emit_to` the session's caller, read authority enforced at publish.

### Batch 2. Sessions

- [x] [Card 205](batch-cards/205-session-teardown-and-entropy.md): teardown API
  wired to window destroy; unguessable-session-id requirement documented (the
  transfer domain already has one); pre-parse byte caps on mutation-reachable
  commands or a recorded reliance on Tauri's transport ceiling.
  **Landed 2026-08-14** — `teardown_window` from the destroyed hook, entropy
  requirement in contract 010, reliance recorded with the bounds-before-work
  rule.

### Batch 3. The copy-path

- [x] [Card 206](batch-cards/206-example-hardening-and-acl-proof.md): the two
  packaged proofs drop `withGlobalTauri`, take a strict CSP, and declare
  Longhorn permission sets; one packaged assertion that a window denied
  `allow-longhorn-bridge-mutate` is actually refused — turning capability
  templates from drift-tested files into behavior-tested policy.
  **Landed 2026-08-14** — `default-src 'self'`, per-window scoped
  permissions, ACL refusals proved through Tauri's real IPC router in
  `src-tauri/tests/acl.rs`. `withGlobalTauri` stays as a named residual (no
  bundler in the proofs by design).

## Dependency Shape

```text
memo 023 (M-events, M-sessions, L-entropy, L-size, L-examples)
 └─ 022 bridge session and event lifecycle
     ├─ 204 event scoping        (decision first)
     ├─ 205 session teardown     (independent)
     └─ 206 example hardening    (independent; adopts 204/205 outcomes if landed)
```

## Goals

- [x] event delivery and read authority agree, by mechanism
- [x] a destroyed window's session stops being valid
- [x] the strongest posture in the repo is the one examples demonstrate —
  strict CSP, per-window ACL; `withGlobalTauri` remains as a named,
  reasoned residual

## Acceptance Criteria

- [x] the event-scoping decision is recorded in contract 010 with its reason
- [x] session teardown is wired to a real window-destroy signal and tested —
  the API and its wiring are in place and lifecycle-tested; the two example
  proofs are the hosts that would wire it, and neither uses the bridge yet
- [x] a denied window is proved refused a mutate command through Tauri's
  real IPC router, not only in a file-drift test

## Explicit Non-goals

- Re-opening the authorization model. Per-route checks, strict grammars, and
  epoch discipline held up under audit; this milestone extends them, it does
  not revise them.

## Next Task

Milestone complete. Next in the audit suite: g02.023 (credential and
activation hardening, Card 207 first) or the independent 211/215/219.

## Planning Checkpoint

After Card 204. Per-window targeting touches the event-sink trait both hosts
implement; if the decision goes that way, check the GPUI seam before
proceeding rather than discovering it mid-205.
