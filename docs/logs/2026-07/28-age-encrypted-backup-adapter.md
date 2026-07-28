# Age Encrypted Backup Adapter

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added optional `longhorn-config-age` without changing the plaintext package
  graph
- pinned `age` 0.12.1 with default features disabled
- added injected `BackupEncryptionProvider` recipient and identity authority
- added redacted X25519 recipient, identity, passphrase, and identity-ring
  types
- added binary age v1 recipient and passphrase envelope creation
- added bounded streaming decryption and strict inner ZIP inspection
- added outer ciphertext and verified inner archive receipts
- added locked, corrupt, unsupported, and authenticated-inner-rejection states
- added active/historical key-ring behavior and explicit re-encryption

## Authority

Operational encryption asks an injected provider for public recipients.
Operational inspection asks the same seam for active and historical
identities. Provider failures carry no arbitrary detail, so a secure-store
adapter cannot accidentally place a private key or passphrase in Longhorn
errors.

Interactive recipient export accepts explicit public keys. Passphrase export
accepts one bounded redacted secret and never routes it into operational
automation. Identities and passphrases implement redacted `Debug` and no
serialization.

## Envelope

The adapter uses the standard streaming age API and produces binary
`age-encryption.org/v1` files. It wraps the complete verified
`.longhorn-backup` ZIP. No sidecar, manifest field, domain name, checksum, or
custom Longhorn cryptographic header is added.

Decryption authenticates the envelope before calling the existing strict ZIP
inspector. Missing or wrong keys return locked. Header/payload damage returns
corrupt. Future age formats and configured size refusals return unsupported.
An authenticated but invalid inner ZIP is reported separately.

Re-encryption borrows the source, authenticates and verifies the inner ZIP,
then creates a fresh target envelope. Target failure produces no replacement
and cannot mutate the source.

## Evidence

- exact `age` 0.12.1 dependency graph compiles and tests on Rust 1.85
- recipient envelopes interoperate in both directions with the raw age API
- passphrase envelopes interoperate in both directions with the raw age API
- ciphertext contains no plaintext manifest path, application id, or domain id
- wrong and unavailable keys are locked, not corrupt
- truncated and modified ciphertext is corrupt before inner inspection
- future age format is unsupported
- authenticated invalid inner bytes never become trusted archive state
- receipt serialization and debug output contain no tested secret material
- rotated active key writes new archives; historical identity reads old ones
- explicit re-encryption uses fresh ciphertext and preserves source on failure
- plaintext retention preserves `.longhorn-backup.age` as locked
- plaintext `longhorn-config` has no age dependency

## Validation

- `cargo +1.85.0 check --workspace` passed
- `effigy qa` passed with 119 Rust tests and every documentation gate
- `effigy doctor` reported 17 warning-level size findings and zero errors
- the passphrase interoperability case takes about six seconds because it
  exercises age's production passphrase work factor

## Boundary

No keychain, secure-store implementation, prompt, recovery phrase, cloud key
custody, ASCII armor, ZIP AES, signing, publication sidecar, settings UI,
consumer write, TypeScript, Svelte, or Poodle dependency was added.

## Posture

`strict-ready`

Card 009 is complete. Card 010 is the sole ready lane and is not auto-started.

## Next

Review and explicitly start card 010 for custom backup adapters and
consumer-shaped conformance.
