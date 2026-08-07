# 157 Tauri Licence Host And Secure Storage

Status: in progress — pure half complete, host wiring outstanding
Owner: Tom
Roadmap: g02.010 batch 3
Governing refs: contracts 019, 004, and 017; research memo 020
Depends on: Card 155
Auto-start next card: no

## Objective

Build `longhorn-tauri-licence`: platform secure storage for credentials, the
RFC 8252 loopback account flow, and licence persistence.

## Scope

- licence state persistence, under the Card 150 store rules
- platform secure storage for tokens and credentials
- system-browser account flow with loopback redirect and PKCE
- machine identity as a random per-installation value

## Steps

1. Persist licence state through the existing configuration store so it
   inherits schema stamping and future-schema refusal. A nightly build must
   not write a licence store production cannot read.
2. Put credentials and refresh tokens in **platform secure storage**, not in
   the configuration store. Longhorn owns this so that consumers do not each
   reimplement it, badly, in their own way.
3. Implement the account flow per RFC 8252: system browser, loopback
   redirect, PKCE. Embedded webviews are not an accepted flow — they break
   password managers and SSO, and users are right to distrust them.
4. Generate machine identity as a random per-installation value. Not a MAC
   address, not a hardware serial, not anything derived from user identity.
   The updater's install identity is the same shape for the same reasons;
   reuse rather than duplicate if the lifetimes match.
5. Wire lease renewal on a schedule, with the fail-open rule intact: an
   unreachable backend within the lease changes nothing the user can see.
6. Surface expiry and renewal state through the diagnostics seam, stamped
   like the update build identity.

## Acceptance Criteria

- licence state refuses a newer schema and leaves the file untouched
- credentials never reach the configuration store or a log
- the account flow opens the system browser; no embedded webview path exists
- machine identity is random per installation and stable across restarts
- lease renewal failure within the lease is invisible to the user
- workspace QA passes

## Evidence Required

- store-refusal test for licence state
- a test or recorded audit that credentials do not reach configuration
  storage
- loopback flow test
- fail-open renewal test

## Stop Conditions

- platform secure storage cannot be reached without a dependency that
  contradicts the workspace dependency posture, in which case the storage
  seam becomes injected and the decision is recorded

## Stop Condition Reached — 2026-08-07

The storage stop condition applies, and the seam is injected as it
prescribes. Binding Longhorn to one keychain crate would contradict the
agnostic posture the operator set for this whole boundary: consuming
applications differ in what they already depend on, and a platform backend
pulls DBus on Linux and Security.framework on macOS into every consumer
whether or not they license anything.

Longhorn therefore owns the **rules** — credentials never enter the
configuration store, never appear in an error, slots are named and
non-colliding — and ships `CredentialStore` plus an in-memory
implementation. A platform backend is composed by the consumer, or supplied
later as an optional feature behind its own card.

Contract 019 says secure storage is "owned by Longhorn so that consumers do
not each reimplement it". The seam plus rules satisfies the intent; the
absence of a shipped platform backend is the part still outstanding.

## Progress

Complete, in `longhorn-licence` and fully tested:

- **PKCE** — `CodeVerifier` with RFC 7636 length and alphabet enforcement,
  S256 challenge verified against the RFC's published test vector rather
  than merely being self-consistent. `plain` is not offered: it sends the
  verifier itself and protects against nothing.
- **Loopback flow** — `AccountFlow` composes the RFC 8252 redirect, and
  `accept_callback` consumes the flow so a replayed redirect cannot be
  exchanged twice. State is compared in constant time and **before** the
  outcome is read, so an unmatched callback learns nothing, not even that a
  flow was in progress.
- **`MachineId`** — random per installation with a minimum length, so a host
  cannot supply a hostname or a counter. Derived from nothing about the
  hardware or the user.
- **`CredentialStore`**, `CredentialSlot`, `MemoryCredentialStore`.

Outstanding, and needing a packaged application to verify:

- `longhorn-licence-config` — the persistence domain carrying the Card 150
  schema-refusal rules
- `longhorn-tauri-licence` — system-browser launch, the loopback listener,
  a platform credential backend, and scheduled lease renewal

## Next Task

The packaged proof application, which Cards 153 and 157 both now wait on,
then Card 158.
