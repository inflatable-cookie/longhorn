# 210 Age-identity Persistence

Status: blocked — operator decision on the store's home
Owner: Tom
Roadmap: g02.023 batch 3
Governing refs: contract 004 (§ noninteractive identity authority); memo 023
(opp-age-slot)
Depends on: Card 208 (settles the credential-slot shape this extends)
Auto-start next card: no

## Objective

The operational age identity gets a Longhorn-owned persistence path, so
automatic encrypted backup stops requiring every consumer to re-implement the
storage decision.

## Why this exists

`AgeIdentity::to_secret()` (`crates/longhorn-config-age/src/authority.rs:69-73`)
hands the secret to the consumer for "an explicit secure-store write", but
`CredentialSlot` (`crates/longhorn-credential-keyring/src/credential.rs:91-97`)
carries only `RefreshToken` and `LicenceKey` — there is no age-identity slot,
and the keyring crate's user field is fixed to those two. Contract 004:403
requires noninteractive identity authority for automatic encrypted backup;
today that requirement has no mechanism.

## Scope

- `crates/longhorn-credential-keyring` — the new slot
- `crates/longhorn-config-age` — the authority side of the seam
- contract 004 — name the mechanism against the requirement
- one consumer-shaped example or test proving the noninteractive path

## The decision, with evidence (2026-08-14)

Execution stopped at the boundary choice, per contract 001. The mechanism is
not in doubt — get-or-generate through the keychain is fifteen lines — but
every placement of it changes what consumers compose:

1. **Move `CredentialStore`/`CredentialSlot` to `longhorn-core`, then add the
   age-identity slot there.** Architecturally cleanest: credential storage is
   host plumbing, not licence domain — it sits in licence only because
   licensing needed it first. Cost: a material consumer break
   (`longhorn_licence::CredentialStore` re-imports from core) across all five
   consumers, inside the 0.1.0 candidate surface.
2. **Add the slot in licence and the provider in the keyring crate.** No
   consumer break, but the platform-backend crate then knows backup-domain
   shapes — off its stated mission.
3. **Add the slot in licence and let config-age depend on licence.** The
   PAPERCUTS precedent says this coupling (two optional capability crates
   that cannot compose separately) is worse than duplication.
4. **Status quo plus a documented recipe.** Consumers keep owning the
   storage decision; Longhorn documents the get-or-generate pattern. Contract
   004:403's "noninteractive identity authority" stays consumer-satisfied.

Recommendation: option 1, coordinated with the next consumer-facing bump —
but it is the operator's call, because it prices a consumer break against a
boundary correction right before the candidate freeze.

## Steps

1. Decide where the identity lives: a third `CredentialSlot` variant, or a
   config-age-owned sidecar with keyring wrapping. The boundary choice is the
   card's substance; contract 004's noninteractive requirement constrains it —
   unattended backup cannot prompt.
2. Implement the slot and the authority-side retrieval.
3. Wire the automatic-backup path to it end to end in a test: no operator
   present, identity resolved, backup encrypted.
4. Amend contract 004:403's mechanism language to name what satisfies it.

## Do Not

- Store the identity in plaintext config. The slot exists because the keyring
  is the boundary.
- Grow `longhorn-core` — this belongs in the two crates that already own the
  halves.

## Acceptance Criteria

- [ ] contract 004:403 names its mechanism
- [ ] the noninteractive backup path runs without an operator in a test
- [ ] the identity never persists outside the chosen secure store

## Evidence Required

- the boundary decision and its reason
- the end-to-end noninteractive test
- `effigy qa` green

## Stop Conditions

Stop if the Linux keychain-absence story (`Unavailable`, documented in the
keyring crate) makes the noninteractive guarantee impossible on a tier the
contract claims — that is a contract-scope question for the operator.
