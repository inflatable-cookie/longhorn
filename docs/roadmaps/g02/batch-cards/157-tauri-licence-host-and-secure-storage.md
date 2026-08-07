# 157 Tauri Licence Host And Secure Storage

Status: ready
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

## Next Task

Card 158.
