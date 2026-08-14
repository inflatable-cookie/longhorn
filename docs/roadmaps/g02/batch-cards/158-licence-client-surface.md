# 158 Licence Client Surface

Status: complete — 2026-08-13
Owner: Tom
Roadmap: g02.010 batch 3
Governing refs: contracts 019, 010, and 013; research memo 020
Depends on: Card 157; Card 193
Auto-start next card: no

## Objective

Build the licence client surface: generated bindings and the Svelte surface for
activation, seat management, and expiry.

## Correction — 2026-08-12

The same two faults Card 154 had, found the same way.

**`packages/licence` cannot be built.** g02.013 consolidated eighteen
TypeScript packages into three and is complete. The surface belongs in
`packages/longhorn/src/licence/`, where every other domain lives.

**There is nothing to bind.** Step 1 presupposes wire types.
`longhorn-licence` derives `ts_rs::TS` four times across three files and has no
`protocol` module, no command envelope, no snapshot and no changed event.
Card 193 supplies them; this card resumes after it.

## Correction — 2026-08-13: step 4 cannot be built

Checked against the tree before starting, as Cards 196 and 159 had to be.

**There is no activation slot list in the protocol.** Card 193 gave the domain
`LicenceDeactivateCommand`, which releases *this machine's* seat, and
`NoSeatsFree` as a rejection code. Neither tells a client which machines hold
seats, so the screen step 4 describes has nothing to render.

`CredentialSlot` in `longhorn-licence` is not it. That is local credential
storage — which keychain entry a secret lives in — and has no relationship to
activated machines.

This is the card's most emphasised step: "the dominant licensing support ticket
… burying it converts every hardware change into a support conversation." It
cannot be buried by this card because it is not here to bury.

**The gap is real work, not an oversight to patch inline.** Listing seats means
the authority reporting machines it has activated: an identity per seat, a
label a human recognises, when it was activated, and which one is *this*
machine. That is a protocol addition with a privacy question attached — a seat
list is a list of a customer's computers — and it wants its own card.

Steps 1, 2, 3, 5, 6 and 7 all have protocol behind them and proceed. Step 4 is
carved out and carded.

## Scope

- generated bindings for the licence domain, in
  `packages/longhorn/src/licence/` as g02.013 left the package graph
- activation: serial key entry, account sign-in, licence-file import
- current licence state, including entitlements and both windows
- releasing this machine's seat; the list of other machines is carved out
- expiry and renewal surfacing

## Steps

1. Generate and check bindings for the licence domain types.
2. Build serial key entry against the Card 156 format helpers: validate
   locally before any round trip, accept wrong case, missing dashes, and
   pasted whitespace. A key that is merely mistyped must never produce a
   message implying the key is invalid.
3. Build account sign-in and licence-file import as peers of key entry, not
   as an advanced fallback. File import is what air-gapped customers use.
4. [x] **Build the activation slot list with self-service release.** Carved
   out on 2026-08-13 because no protocol exposed the seats, then restored the
   same day: Card 199 added `LicenceSeatProjection` and
   `LicenceReleaseSeatCommand`, and the controller exposes `seats` and
   `otherSeats`. The rendering is Poodle's.
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
- releasing this machine's seat is reachable without contacting support. The
  slot *list* moved to its own card and is not claimed here.
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

## Outcome — 2026-08-13

Complete on both sides. Longhorn holds validation, ports, client, controller
and the key-format helpers; Poodle renders `LicenceStatus`, `LicenceSeats` and
`LicenceActivation`; and three binding components join them in
`longhorn-poodle-svelte/src/licence/poodle/`, over the same sampler the update
surface uses.

**The key format exists exactly twice, and Poodle is not one of them.** Their
`LicenceActivation` takes a `LicenceKeyFormat` interface and the binding
injects Longhorn's `parseLicenceKey` and `isProbablyATypo` — so a mistyped key
fails locally, on submit, with their copy: "Check the key for a typing
mistake.", documented in their source as never `invalid`, `fake`, or
`not recognised`. The generated conformance fixture binds the Rust and
TypeScript implementations, and was verified to bite by breaking one confusable
mapping.

**Three binding decisions, each the honest side of a gap:**

- ~~`onRename` is offered by Poodle's seat list and deliberately unwired.~~
  Closed 2026-08-14 by operator decision: the protocol gained
  `LicenceRenameSeatCommand` — machine id plus a nullable label, following
  activation's rules, with `seatRelabelled` as its changed kind — and the
  binding wires it. One normalisation lives at the seam on purpose: Poodle's
  editor can commit an empty string for a cleared field, the protocol refuses
  empty as a mistake wearing null's clothes, and the binding maps
  whitespace-and-empty to null so each side keeps its own honest rule.
  The seam went from flagged to closed in a day, in the right order:
  capability offered, protocol caught up, binding wired.
- The account route's token provider rejects with "account sign-in is not
  composed yet" rather than silently doing nothing. It waits on the platform
  `CredentialStore` decision, as Card 159's licence half does.
- Unlicensed renders no status panel and no seat accounting renders no list.
  "0 machines" would imply an accounting that is not happening.

Two test corrections were mine, not theirs: their expired copy is "use coverage
ended" rather than "expired", and they validate on submit rather than on input
— which is right, because mid-typing validation flags every key as mistyped
until its final character.
