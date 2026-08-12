# 190 Update Protocol Surface

Status: complete — steps 1-3 landed 2026-08-12, step 4 on 2026-08-13
Owner: Tom
Roadmap: g02.009 batch 3
Governing refs: contract 018; contracts 010, 011, 012; research memo 019
Depends on: Card 151 (complete); Card 152 (complete); Card 153 (complete)
Blocks: Card 154
Auto-start next card: no

## Why

Card 154 was written to build the update client surface, and its first step is
"generate and check bindings for the update domain types". There are none.

`longhorn-update` is a finished domain — channel resolution, semver comparison,
rollout staging, deferral, source adapters, the quiescence gate — and none of
it crosses a boundary. It derives `ts_rs::TS` on seven enums (`Channel`,
`CheckKind`, `OfferReason`, `DeferralCause`, `InstallProvenance`,
`InstallManager`, `QuiescenceKind`) and on nothing else.

The gap is narrower than "no protocol" and wider than "add some derives".
`UpdateAvailability` is already a `#[serde(tag = "state")]` union, and
`UpdateOffer`, `Deferral`, `OutstandingWork` and `QuiescenceReceipt` all
serialise. What is missing is the envelope every other domain has:

| | Update | Every other domain |
| --- | --- | --- |
| Versioned command | — | `protocolVersion`, `authorityEpoch`, `expectedRevision` |
| Snapshot projection | — | one type a client reads state from |
| Changed event | — | `*ChangedEvent` with a kind |
| Generated field maps | — | flat and per-variant |

## Scope

`longhorn-update`, the bindings generator, and the Tauri host crate. The client
surface is Card 154 and stays there: this card stops at "a consumer could
build one".

Progress is the part with no representation at all today, so it is step 3 on
its own rather than a line in step 1.

## Step 1 — The snapshot

- [ ] `UpdateSnapshot`: protocol version, authority epoch, the selected
      `Channel`, the installed `BuildIdentity`, the last check's
      `UpdateAvailability`, and the current `Deferral` if any.
- [ ] `AheadOfChannel` survives into the projection as its own state, not
      folded into up-to-date. Card 154 step 5 calls this the single most likely
      support question the feature generates, and a projection that loses the
      distinction makes that surface impossible to build.
- [ ] The crate owns no clock, so "when was the last check" is a host-supplied
      stamp or absent — the same rule and the same type as Card 182's
      `HistoryRecordedAt`, reused rather than re-invented.

## Step 2 — The commands

Four, matching the four things an operator can do.

- [ ] `UpdateCheckCommand` — ask the source now.
- [ ] `UpdateSelectChannelCommand` — carries the target `Channel`.
- [ ] `UpdateDeferCommand` — carries the `DeferralCause`.
- [ ] `UpdateInstallCommand` — authorize the install. Card 153 settled that
      Longhorn authorizes and the application installs, so this returns the
      authorization and the quiescence receipt, not an installed state.
- [ ] Each carries the standard envelope and is refused on a stale
      `expectedRevision`, as every other domain's command is.

## Step 3 — Progress has no representation

Nothing in the crate models a download or an install in flight. Card 154 needs
it and cannot invent it client-side without holding state the authority does
not have.

- [ ] `UpdateProgress`: a tagged union over idle, downloading with a fraction,
      verifying, ready-to-install, and installing.
- [ ] A fraction is `Option`. A source that does not report content length
      cannot produce one, and a bar that invents a number is worse than a bar
      that says it does not know.
- [ ] The authority reports progress; it does not perform the download. The
      host drives the transfer and reports, the same division Card 153 set for
      installation.

## Step 4 — The changed event and the surfaces

- [x] `UpdateChangedEvent` with a kind, so a consumer invalidates without
      polling. Follow `ForkChangedEvent`.
- [x] Register the domain in the bindings generator, with both field maps —
      flat and per-variant. The per-variant map matters here: this domain is
      mostly tagged unions, and `UpdateAvailability` is tagged `state`, which
      is exactly the case Card 188's detector was built for.
- [x] Tauri commands with named re-exports, per Card 181 step 2. `check` and
      `install` are separate capabilities: authorizing an install is not
      covered by permission to look for one.

## Acceptance

- [ ] `effigy qa` passes, including `check:bindings`.
- [ ] A round-trip test per command and per projection, as the other domains
      have.
- [ ] A test asserts `AheadOfChannel` projects distinctly from `UpToDate`, by
      state name, not by an absent field.
- [ ] A test asserts a download with no content length projects a `null`
      fraction rather than zero.
- [ ] A stale `expectedRevision` is refused on all four commands.
- [ ] The generator reports no unreadable union in the update domain.

## Evidence

- [ ] The tests above, named in the batch log.
- [ ] The generated TypeScript, showing the four commands, the snapshot, the
      progress union and the event.

## Stop Conditions

- Stop if progress cannot be modelled without the authority knowing how the
  host performs the download. Card 153 drew that line deliberately and a
  protocol that crosses it makes Longhorn responsible for transfers it does
  not run.
- Stop if the snapshot needs a clock to be useful. `recorded_at` is
  host-supplied by the same rule as Card 182, and a projection that has to
  invent a time is a modelling gap rather than a missing field.

## Continuation

Card 154, rescoped: `packages/longhorn/src/update/` and
`packages/longhorn-poodle-svelte/src/update/`, not `packages/update`.

## Outcome — 2026-08-12

Steps 1 to 3 landed: a versioned line, four commands, the snapshot, the install
authorization, the changed event, and the progress union that had no
representation anywhere. Eight round-trip tests, `effigy qa` exit 0.

Three departures from the card, each recorded in the type it affects. Versions
are strings, because `semver::Version` already serialises as one. The authority
epoch is a plain `u64`, as `operation` and `notifications` carry it, rather than
pulling `longhorn-history` in for one integer. And there is no timestamp: the
card wanted Card 182's host-supplied stamp reused, that type lives in
`longhorn-history`, and no surface this protocol exists for asks for a time.

**Step 4 was stopped, and then unblocked.**

It was stopped because Card 153 recorded that Tauri's plugin performs check,
download, verification and install, which left three things unanswered: there
was no host crate for the commands, Card 152's source adapters overlapped the
plugin's check, and progress looked like a pass-through the authority should
not hold.

The operator decision of 2026-08-12 answers all three by removing the plugin:
Longhorn is the update controller for both hosts. So `UpdateCheckCommand` is
Longhorn's, progress is observed rather than relayed, and the source adapters
are the only check path. See g02.009 for what that makes Longhorn responsible
for -- signature verification most of all.

Step 4 itself now needs a host crate that does not exist, because
`longhorn-tauri-update` was absorbed and its tauri dependency deliberately
removed. Recreating it is a decision for the card that does the install work,
not this one.


## Outcome, Step 4 — 2026-08-13

`longhorn-tauri-update` exists again. A crate of that name was absorbed into
`longhorn-update` on 2026-08-09 and its tauri dependency deliberately removed;
what it held then was the installer, and what it holds now is only the seam —
commands, capabilities, and the invalidation hint. A test asserts it depends on
neither `longhorn-update-install` nor minisign, so the distinction cannot erode
quietly.

Five commands over four capabilities, which is one more split than the card
asked for.

| Capability | Commands | Why separate |
| --- | --- | --- |
| read | `snapshot` | Local. The last answer, already computed. |
| check | `check` | Reaches the network on the operator's behalf. |
| mutate | `select_channel`, `defer` | Changes what this install follows. |
| install | `install` | Replaces the running application. |

The card required `check` and `install` to be separate. Splitting `read` from
`check` as well follows the same rule one step further: a window that displays
update state has not thereby been given permission to make requests. A test
asserts every command is granted by exactly one permission and that the count
matches the crate, which is the check Card 183 wished existed when a fork
command shipped without one.

Only a committed outcome publishes an invalidation hint. A rejection leaves the
state as it was, so a consumer that refetched on one would be refetching for
nothing.

**Two things were added ahead of Card 154, so the seam could be typed.**
`packages/longhorn/src/update/index.ts` re-exports the generated protocol and
both field maps, and the package gained `./update` and `./update/protocol`
exports. No client, no controller, no validation — those are Card 154.

The port returns `Promise<unknown>` from every call, as every other raw port
does. What comes back over a transport is untrusted until a validator says
otherwise, and the checked port that narrows these arrives with the validation
that earns it. Commands going out are typed, because those this side builds.
