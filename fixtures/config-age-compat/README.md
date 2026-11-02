# Retained config-age ciphertext fixtures

Frozen encrypted-backup ciphertext captured through the `longhorn-config-age`
public API while the workspace still resolved age 0.12.1 with age-core 0.12.0 —
the dependency line Longhorn 0.4.3 shipped and the 0.5.0 candidate
(1e1f570c88091b7a9196438bc05e140071babc33) froze against. They are the
stored-data compatibility evidence for the RUSTSEC-2026-0387 repair
(crypto-bigint 0.5.5 via age 0.12.1's mandatory hpke → p256 → elliptic-curve
path): `tests/age_envelope/compat.rs` proves this exact ciphertext still
restores after the dependency line moves to age 0.11.5.

## Files

| File | Content |
| --- | --- |
| `recipient.age` | X25519 recipient envelope (`encrypt_export_to_recipients`) |
| `recipient-identity.txt` | The envelope's secret identity, committed deliberately |
| `passphrase.age` | Human-passphrase envelope (`encrypt_export_with_passphrase`) |
| `manifest.json` | Provenance and SHA-256 digests the compat tests assert |

## Why a secret key is committed

Both envelopes protect only a synthetic example archive captured by
`longhorn-config-age`'s own test support. The identity guards no real data;
without the identity in the tree, the X25519 fixture would be undecryptable
and could prove nothing. The passphrase is a fixed test constant in
`tests/age_envelope/compat.rs`. Never replace these files with ciphertext
captured by the current dependency line: regeneration silently defeats the
retained-artifact property. The one-time capture was performed by a generator
test that was deleted after producing these files, at the commit named in
`manifest.json`.
