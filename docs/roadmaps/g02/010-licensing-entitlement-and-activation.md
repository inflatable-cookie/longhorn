# g02.010 Licensing, Entitlement, And Activation

Status: complete — 2026-08-14
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

Tenth g02 task, and the second non-remediation one. Shares the adapter
posture of g02.009 deliberately: `ActivationSource` is the same shape as
`UpdateSource`, so a consumer who has integrated one already knows the
other.

The two milestones meet at the update window — the updater asks the licence
whether a release may be taken.

## Execution Plan

### Stage 1. Licence core

- [x] Card 155
  builds `longhorn-licence`: verified licence, trust basis, entitlements,
  the two windows, lease and grace, clock-regression refusal

### Stage 2. Acquisition

- [x] Card 156 defines
  `ActivationSource` and the signed-file and token-redemption reference
  adapters

### Stage 3. Host and surface

- [x] Card 157 (complete 2026-08-14)
  builds `longhorn-tauri-licence`: secure credential storage, the RFC 8252
  loopback flow, persistence — PKCE, callback validation, machine identity
  and the credential seam are complete; browser launch, platform backend and
  persistence need a packaged proof application
- [x] Card 159 (complete; shared packaged proof)
  builds the shared packaged proof application both host cards wait on
- [x] Card 193 (complete
  2026-08-12) gives the licence domain a wire protocol — snapshot, commands, rejection codes, changed event —
  which Card 158 assumed existed
- [x] Card 158 builds the licence (Longhorn side complete 2026-08-13)
  client surface: activation, expiry, entitlement reads. Longhorn side complete
  2026-08-13; step 4 carved out because no protocol exposes the seats, and the
  Svelte rendering is Poodle's
- [x] Card 199 (complete 2026-08-13) gives the
  protocol a seat list, so a customer who has changed laptop can release the
  old one without a support conversation

## Dependency Shape

```text
memo 020 licensing
 └─ 155 licence core ─┬─ 156 activation adapters
                      └─ 157 host and storage ─ 158 client surface
```

156 and 157 are independent of each other. The task is independent of
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
- [x] licence state refuses a newer schema, per Card 150
- [x] no crate exposes an enforcement call

## Explicit Non-goals

- a licence server, payment handling, tax handling
- signing key custody
- hardware fingerprinting
- EULA presentation
- obfuscation or anti-tamper

## Next Task

None. The task is complete.

An application can hold a licence and answer "entitled?" without enforcing;
activate by key, account sign-in or file with a mistyped key never reading as
an invalid one; keep credentials in the platform keychain with locked never
reading as absent; renew a lease that fails open within grace; and show a
customer their seats and free the old laptop without a support conversation.
The claims that needed a machine and a human were observed rather than
asserted: cross-process keychain persistence, the locked-keychain denial, and
a sign-in through the real system browser.

Consumer adoption — composing the authority behind `LicenceHostAuthority` in
Soundcheck or Nucleus — follows separately, as update adoption does.

Previously: the two operator steps, Cards 157/158/199, Card 193.

## Absorbed records

Absorbed from `batch-cards/` in the flattened-task migration; the directory is removed and git history is the full-fidelity archive.

- Card 155: complete — licence model and entitlement evaluation.
- Card 156: complete — activation source adapters.
- Card 157: complete 2026-08-14 — Tauri licence host and secure storage (checkbox staleness corrected in migration).
- Card 158: complete 2026-08-13 — licence client surface (checkbox staleness corrected in migration).
- Card 159: complete — shared packaged proof (licence half 2026-08-14; update half with g02.009).
- Card 193: complete 2026-08-12 — licence protocol surface.
- Card 199: complete 2026-08-13 — seat list protocol (checkbox staleness corrected in migration).
