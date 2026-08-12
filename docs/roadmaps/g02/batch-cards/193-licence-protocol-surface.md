# 193 Licence Protocol Surface

Status: ready
Owner: Tom
Roadmap: g02.010 batch 3
Governing refs: contract 019; contracts 010, 012; research memo 020
Depends on: Card 155 (complete); Card 156 (complete)
Blocks: Card 158
Auto-start next card: no

## Why

Card 158 was written to build the licence client surface, and its first step is
"generate and check bindings for the licence domain types". There are none —
the same finding as Card 154, in the same place, for the same reason.

`longhorn-licence` derives `ts_rs::TS` four times across three files
(`Usability`, `Entitlements`' id type, and two in `time`). It has no `protocol`
module, no command envelope, no snapshot and no changed event.

`Usability` is already a `#[serde(tag = "state")]` union and several types
serialise, so the gap is narrower than "no protocol" and wider than "add
derives" — exactly as it was for update.

## The Pattern, Recorded Once

Both milestones' batch-3 cards say "build `packages/X`". g02.013 consolidated
eighteen TypeScript packages into three and is complete, so neither
`packages/update` nor `packages/licence` can be built; both surfaces belong in
`packages/longhorn/src/<domain>/`.

Both also assume a wire protocol their domain never grew. Card 190 supplied
update's; this supplies licence's.

Checked 2026-08-12: those are the only two live cards naming a package that no
longer exists. Every other reference is inside a completed card, describing
what was true when it ran. This is not a sweep.

## Scope

`longhorn-licence` and the bindings generator. The client surface is Card 158.
The Tauri host is Card 157, which is separately blocked on Card 159.

## Step 1 — The snapshot

A client needs to answer "what am I entitled to, and for how long" without
learning how a licence is proved.

- [ ] `LicenceSnapshot`: protocol line, authority epoch, the `Usability`
      projection, the entitlement ids held, and both windows — use and update —
      as timestamps.
- [ ] `Usability` projects every variant distinctly, including `ClockRefused`.
      A licence refused because the machine clock moved is not expired, and a
      surface that shows "expired" for it sends the operator to buy something
      they already own.
- [ ] Entitlements are **opaque ids**. Longhorn enumerates no features, per the
      milestone's own acceptance criterion, and the protocol must not become
      the place that does.
- [ ] The trust basis is reported but never the credential. A client may show
      "verified offline" versus "confirmed with the server"; it may not receive
      a signature, a token, or a key.

## Step 2 — The commands

Card 158 lists three activation routes as peers: serial key, account sign-in,
licence-file import.

- [ ] One `LicenceActivateCommand` carrying a tagged credential, not three
      commands. They are three ways to present the same thing, and three
      commands would make the client choose a code path where the authority
      should.
- [ ] `LicenceDeactivateCommand` for self-service seat release.
- [ ] `LicenceRefreshCommand` to re-check the lease.
- [ ] Each carries the standard envelope and is refused on a stale epoch.

## Step 3 — Rejections a client can act on

The milestone's evidence requires that a mistyped key never reads as an invalid
one. That distinction has to survive the wire.

- [ ] A rejection projection with a code, distinguishing at least: malformed
      input, not recognised, no seats free, revoked, and clock refused.
- [ ] Card 156's format helpers validate locally first, so malformed should
      rarely reach the authority. It still needs the code, because a client
      that cannot tell "wrong shape" from "wrong key" writes one message for
      both.

## Step 4 — The changed event and the surfaces

- [ ] `LicenceChangedEvent` with a kind, following `ForkChangedEvent`.
- [ ] Register the domain in the bindings generator, with both field maps.
      `Usability` is tagged `state` and the credential will be tagged too, so
      Card 188's detector earns its keep again.
- [ ] No Tauri commands here. Card 157 owns the host and is blocked on
      Card 159; adding them would put a second unfinished thing in this card.

## Acceptance

- [ ] `effigy qa` passes, including `check:bindings`.
- [ ] A round-trip test per command and per projection.
- [ ] A test asserts `ClockRefused` projects distinctly from every expiry
      state, by state name.
- [ ] A test asserts no credential material appears in any projection — the
      same shape as the payload-free assertions the history proofs make.
- [ ] The generator reports no unreadable union in the licence domain.

## Evidence

- [ ] The tests above, named in the batch log.
- [ ] The generated TypeScript, showing the commands, the snapshot, the
      rejection codes and the event.

## Stop Conditions

- Stop if the snapshot cannot express entitlements without naming a feature.
  Longhorn answering "entitled?" is the milestone's line, and a protocol that
  enumerates features crosses it permanently.
- Stop if a rejection code cannot be produced without revealing whether a key
  exists. "Not recognised" and "revoked" are different to an operator and may
  be the same to an attacker, and which to send is the operator's call.

## Continuation

Card 158, rescoped as Card 154 was: `packages/longhorn/src/licence/`, not
`packages/licence`.
