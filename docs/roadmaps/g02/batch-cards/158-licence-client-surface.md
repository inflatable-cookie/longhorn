# 158 Licence Client Surface

Status: ready
Owner: Tom
Roadmap: g02.010 batch 3
Governing refs: contracts 019, 010, and 013; research memo 020
Depends on: Card 157
Auto-start next card: no

## Objective

Build `packages/licence`: generated bindings and the Svelte surface for
activation, seat management, and expiry.

## Scope

- generated bindings for the licence domain, living with this package
- activation: serial key entry, account sign-in, licence-file import
- current licence state, including entitlements and both windows
- activation slot list with self-service release
- expiry and renewal surfacing

## Steps

1. Generate and check bindings for the licence domain types.
2. Build serial key entry against the Card 156 format helpers: validate
   locally before any round trip, accept wrong case, missing dashes, and
   pasted whitespace. A key that is merely mistyped must never produce a
   message implying the key is invalid.
3. Build account sign-in and licence-file import as peers of key entry, not
   as an advanced fallback. File import is what air-gapped customers use.
4. **Build the activation slot list with self-service release.** "I got a
   new laptop" is the dominant licensing support ticket; this screen is the
   feature that answers it, and burying it converts every hardware change
   into a support conversation.
5. Surface both windows distinctly. "Your subscription lapsed" and "your
   updates lapsed but the app keeps working" are different messages, and
   conflating them on a perpetual licence reads as the app breaking.
6. Surface lease state honestly without alarming: a renewal that has not yet
   succeeded, but is inside its lease, is not a problem the user needs to
   act on.
7. Never present enforcement. The surface reports entitlement state; what a
   missing entitlement does is the application's.

## Acceptance Criteria

- a mistyped key fails locally with a message that says so
- activation slot release is reachable without contacting support
- the use window and the update window are distinguishable in the surface
- an in-lease renewal failure does not present as an error
- bindings check clean against the Rust surface
- peers stay peers; no hidden duplicate runtime

## Evidence Required

- per-state rendering tests, including in-lease renewal failure
- key-entry acceptance and rejection tests
- bindings check receipt

## Stop Conditions

- the surface cannot express both windows without assuming a purchase model

## Next Task

Close g02.010.
