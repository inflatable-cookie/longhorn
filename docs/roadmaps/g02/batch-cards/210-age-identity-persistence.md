# 210 Age-identity Persistence

Status: ready
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
