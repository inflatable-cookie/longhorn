# g02.010 Licensing, Entitlement, And Activation

Status: ready
Owner: Tom
Updated: 2026-08-07
Governing refs: contract 019; contracts 004 and 018; research memo 020
Depends on: none within g02

## Outcome

Consuming applications can sell licences on whatever model and whatever
backend they choose. Longhorn owns the licence shape, verification,
entitlement evaluation, lease and grace, and the client surface. It ships no
server and enumerates no features.

## Generation Runway

Tenth g02 milestone, and the second non-remediation one. Shares the adapter
posture of g02.009 deliberately: `ActivationSource` is the same shape as
`UpdateSource`, so a consumer who has integrated one already knows the
other.

The two milestones meet at the update window — the updater asks the licence
whether a release may be taken.

## Execution Plan

### Batch 1. Licence core

- [x] [Card 155](batch-cards/155-licence-model-and-entitlement-evaluation.md)
  builds `longhorn-licence`: verified licence, trust basis, entitlements,
  the two windows, lease and grace, clock-regression refusal

### Batch 2. Acquisition

- [x] [Card 156](batch-cards/156-activation-source-adapters.md) defines
  `ActivationSource` and the signed-file and token-redemption reference
  adapters

### Batch 3. Host and surface

- [ ] [Card 157](batch-cards/157-tauri-licence-host-and-secure-storage.md)
  builds `longhorn-tauri-licence`: secure credential storage, the RFC 8252
  loopback flow, persistence — PKCE, callback validation, machine identity
  and the credential seam are complete; browser launch, platform backend and
  persistence need a packaged proof application
- [ ] [Card 159](batch-cards/159-update-and-licence-packaged-proof.md)
  builds the shared packaged proof application both host cards wait on
- [x] [Card 193](batch-cards/193-licence-protocol-surface.md) (complete
  2026-08-12) gives the licence domain a wire protocol — snapshot, commands, rejection codes, changed event —
  which Card 158 assumed existed
- [ ] [Card 158](batch-cards/158-licence-client-surface.md) builds the licence
  client surface: activation, expiry, entitlement reads. Longhorn side complete
  2026-08-13; step 4 carved out because no protocol exposes the seats, and the
  Svelte rendering is Poodle's
- [ ] [Card 199](batch-cards/199-which-machines-hold-a-seat.md) gives the
  protocol a seat list, so a customer who has changed laptop can release the
  old one without a support conversation

## Dependency Shape

```text
memo 020 licensing
 └─ 155 licence core ─┬─ 156 activation adapters
                      └─ 157 host and storage ─ 158 client surface
```

156 and 157 are independent of each other. The milestone is independent of
g02.009 except at the update window, which Card 155 models and the updater
reads.

## Goals

- [x] every purchase model expressible without a Longhorn change
- [x] entitlements opaque; Longhorn enumerates no features
- [x] trust basis recorded, and offline grace never granted on a basis that
  cannot survive being offline
- [x] Longhorn answers "entitled?" and never enforces
- [x] an unreachable backend fails open within the lease

## Acceptance Criteria

- [x] subscription, perpetual-with-maintenance, trial, and freemium are all
  expressed in tests using only the two windows, with no model-specific code
- [x] a remote-assertion licence cannot obtain offline grace reserved for
  offline-verifiable licences
- [x] a consumer-implemented adapter inherits evaluation with no extra
  wiring
- [x] a large backwards clock movement is refused
- [ ] licence state refuses a newer schema, per Card 150
- [x] no crate exposes an enforcement call

## Explicit Non-goals

- a licence server, payment handling, tax handling
- signing key custody
- hardware fingerprinting
- EULA presentation
- obfuscation or anti-tamper

## Next Task

Two operator steps, and then this milestone is a closure note.

Cards 155, 156, 157, 158, 193 and 199 are complete. The licence system runs
end to end: rules, protocol, seat list, key format twice-and-bound, client
surface both sides, keychain storage with cross-process persistence proved,
the RFC 8252 chain composed over a real socket, and the Tauri seam with
activation as its own credential-bearing grant.

What remains is Card 159's licence half, and both items are operator actions
rather than code:

1. **The locked-keychain path.** `security lock-keychain`, then open the
   licence surface: it must say the store is unavailable, not that the machine
   is unactivated. One minute, and it is the seat-churn defence observed
   rather than asserted.
2. **A sign-in through the real system browser.** Every piece below the click
   is proved; the click needs a packaged run and a human.

Consumer adoption — Soundcheck or Nucleus composing the authority behind
`LicenceHostAuthority` — follows separately, as update adoption does.

Previously: Card 199, then Cards 158 and 157.
