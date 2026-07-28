# 009 Age Encrypted Backup Adapter

Status: complete
Owner: Tom
Roadmap: g01.002 batch 3
Governing refs: contracts 001, 004, and 012; research memo 006
Auto-start next card: no

## Objective

Add an optional whole-archive binary age v1 adapter with injected key
authority, recipient and passphrase export, locked inspection, rotation
behavior, and no secret material in ordinary configuration.

## Scope

- Rust 1.85-compatible age implementation characterization
- replaceable encryption-provider trait
- noninteractive operational recipient and identity flow
- interactive recipient-key and passphrase export inputs
- streaming inner archive encryption and decryption
- outer ciphertext and inner archive receipts
- locked versus corrupt versus unsupported envelope state
- active and historical identity ring behavior
- explicit decrypt-and-reencrypt rotation operation
- retention refusal while authentication and manifest inspection are
  unavailable

## Public Behavior

Encryption accepts a complete verified inner archive and produces binary age
v1. Decryption produces a private staged inner archive and runs the same card
006 inspection. No manifest or payload path is copied to an unencrypted
sidecar.

Providers own secure-store access and prompting. Longhorn handles supplied
recipient and identity objects but never serializes private keys or
passphrases into config, manifests, logs, errors, or receipts.

Operational automation fails when no noninteractive recipient is available.
An unavailable identity returns locked state. Rotation changes new recipient
sets; old archives remain readable only while an old identity stays available.

## Out Of Scope

- designing a new cipher, KDF, or envelope
- ZIP AES, ASCII armor, signing, cloud key management, or key escrow
- settings UI and recovery phrase UX
- external data snapshot adapters

## Acceptance Criteria

- recipient and passphrase round trips interoperate with age v1
- encrypted bytes expose no plaintext manifest or domain names
- wrong or unavailable identity is locked/authentication failure, not archive
  corruption
- truncated or modified ciphertext fails before inner archive trust
- passphrases and private identities never appear in serialized evidence or
  debug output
- operational encryption refuses interactive-only authority
- rotated active key writes new archives while historical identity reads old
  archives
- explicit re-encryption creates a new envelope and leaves source intact on
  failure
- retention preserves a locked archive
- core plaintext backup/restore remains usable without the optional adapter
- Rust 1.85 remains supported

## Stop Conditions

- selected dependency raises MSRV
- implementation requires a custom cryptographic format
- automatic backup needs a stored plaintext passphrase
- sidecar metadata exposes the encrypted manifest
- retention can delete an unauthenticated locked archive
- the card expands into cloud key custody or UI

## Next Task

Card 010 is ready. Do not auto-start it.
