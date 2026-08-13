# 199 Which Machines Hold A Seat

Status: ready
Owner: Tom
Roadmap: g02.010 batch 3
Governing refs: contract 019; research memo 020
Depends on: Card 193 (complete)
Blocks: Card 158 step 4
Auto-start next card: no

## Why

**"I got a new laptop" is the dominant licensing support ticket, and Longhorn
cannot answer it.**

Card 158 step 4 asks for an activation slot list with self-service release.
Checked 2026-08-13: no protocol exposes the seats. Card 193 gave the domain
`LicenceDeactivateCommand`, which releases *this machine's* seat, and
`NoSeatsFree`, which says every seat is taken. Between them a customer can
learn they are out of seats and can free the one they are sitting at — and
cannot see, or release, the laptop they no longer own.

`CredentialSlot` in `longhorn-licence` is not this. It names which keychain
entry a secret lives in and has no relationship to activated machines.

Card 158's other six steps built. This one had nothing to build against, so it
was carved out rather than quietly dropped.

## What It Costs

Every hardware change becomes a support conversation. That is the failure
Card 158 names, and it is worse than it sounds: the customer hits
`NoSeatsFree` at the moment they are trying to start work on a new machine,
and the only path out runs through a human.

## The Decision This Card Carries

**A seat list is a list of a customer's computers.** That is the whole
difficulty, and it is a privacy question before it is a protocol one.

- **What identifies a seat.** `MachineId` exists in `longhorn-licence`. Whether
  the *authority* should retain it, and whether it should come back to the
  client, is not obvious — an id that is stable enough to release is stable
  enough to correlate.
- **What labels a seat.** A customer needs to recognise "the old MacBook", and
  a hostname is both the useful answer and personal data. A user-supplied label
  set at activation is friendlier to privacy and worse at recognition when
  nobody set one.
- **What the list omits.** Last-seen times make the list far more usable and
  turn it into a record of when someone was working. The default should be to
  carry less than feels natural.
- **Whether releasing another seat needs proof.** Releasing the machine you are
  on is self-evidently yours to do. Releasing a different one is an action
  against a machine that is not in front of you.

## Scope

`longhorn-licence`'s protocol, the bindings, and Longhorn's client surface.
Not the Svelte rendering, which is Poodle's, and not the authority's storage,
which is the licensing backend's.

## Steps

- [ ] Settle the four questions above before adding a field. A seat list is
      easy to design and hard to un-ship.
- [ ] Add the projection: the seats held, which one is this machine, and
      whatever identity and label survive the decisions above.
- [ ] Add a command that releases a *named* seat, distinct from
      `LicenceDeactivateCommand`. Same reasoning as the fork-deletion
      capability in g02.018: destroying something you are not standing in is a
      different act from leaving.
- [ ] Register in the bindings generator; the variant map already covers the
      domain's four unions and will cover a fifth.
- [ ] Extend the client surface and the controller.
- [ ] Resume Card 158 step 4.

## Acceptance

- [ ] `effigy qa` passes.
- [ ] A test asserts the seat list carries no credential material, by the same
      assertion the rest of the domain uses.
- [ ] A test asserts this machine is identifiable in the list without the
      client comparing identifiers it had to be told separately.
- [ ] Releasing a named seat is refused on a stale authority epoch, as every
      other command in the domain is.
- [ ] Whatever the privacy decisions were, they are recorded in the types
      rather than in this card alone.

## Evidence

- [ ] The tests above, named in the batch log.
- [ ] The four decisions, with what was rejected and why.

## Stop Conditions

- Stop if the list cannot be built without the authority retaining more about
  a customer's machines than releasing a seat requires. A support conversation
  is a smaller cost than a fleet inventory nobody asked for, and this card
  should not talk itself into one.

## Continuation

Card 158 step 4, then g02.010's remaining batch-3 work.
