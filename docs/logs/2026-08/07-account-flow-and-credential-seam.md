# Account Flow And Credential Seam

Date: 2026-08-07
Card: 157 (partial)
Roadmap: g02.010

## Result

The pure, security-critical half of the licence host lands in
`longhorn-licence`: PKCE, RFC 8252 callback validation, machine identity,
and the credential seam. 62 crate tests. The host wiring stops at claims
that cannot be verified headlessly, and Card 159 now exists to carry them.

## Shape

- `CodeVerifier` — RFC 7636 length and alphabet enforcement, S256 challenge.
- `AccountFlow` — loopback redirect composition, `accept_callback`.
- `MachineId` — random per installation, minimum length.
- `CredentialStore`, `CredentialSlot`, `MemoryCredentialStore`.

## Decisions

**The S256 challenge is checked against RFC 7636's published test vector.**
A self-consistent implementation proves nothing: an encoding mistake that
hashes and encodes the same way on both sides passes every round-trip test
and fails against every real authorization server.

**`plain` is not offered.** RFC 7636 defines it, and it sends the verifier
itself, so it protects against nothing an interception can do. Offering a
mode with no security property invites someone to select it.

**State is compared in constant time and before the outcome is read.** A
callback that did not come from this flow gets `StateMismatch` and nothing
else — not even the knowledge that a denial was in flight. Tested directly,
because the natural way to write the function reads the outcome first.

**`accept_callback` consumes the flow.** A replayed redirect cannot be
exchanged twice, by type rather than by a flag someone forgets to set.

**Machine identity has a minimum length and no derivation.** It is random,
per installation, and not the hardware, the network adapter, or the user.
The minimum stops a host quietly supplying a hostname or a counter.

**The credential seam is injected, and this is the card's stop condition
firing as designed.** Binding Longhorn to one keychain crate would pull DBus
on Linux and Security.framework on macOS into every consumer whether or not
they license anything, and would contradict the agnostic posture set for
this whole boundary. Longhorn owns the rules — credentials never enter the
configuration store, never appear in an error, slots do not collide — and
ships the trait plus an in-memory implementation. A platform backend is
composed, or lands later behind its own card.

Credentials stay out of configuration storage because that store is
world-readable, is included in backups, and is copied between machines on
restore. A refresh token in a backup archive is a leak with a long tail.

## Outstanding

`longhorn-licence-config` and `longhorn-tauri-licence`. Both need a
packaged application: a real keychain including the locked path, a real
browser redirect, and persistence proved against the Card 150 rules.

## Notes

Card 153 stopped at the same boundary for the same reason. Two cards blocked
on one absent thing means the absent thing is a card, so
`159-update-and-licence-packaged-proof` now carries both, following the
repository's existing native-content proof conventions.

`cargo fmt --check` clean, clippy clean on both feature passes, full
workspace suite green.
