# g02.009 Application Update And Release Channels

Status: complete — 2026-08-13
Owner: Tom
Updated: 2026-08-07
Governing refs: contract 018; contracts 004, 012, and 017; research memo 019
Depends on: none within g02

## Outcome

In-app update for consuming applications: an application notices a new
release on its selected channel, offers it, and installs it without losing
work in flight. Longhorn owns channel policy, client-side rollout, source
adapters, and restart readiness. Hosting, signing, and installation stay
outside the boundary.

## Generation Runway

Ninth g02 milestone, and the first that is not remediation.

Card 150 was compiled as a gate on the belief that no store records the
schema that wrote it. That was wrong — all four stores already stamp and
refuse forward, and the write path refuses to overwrite a store it could not
load. Card 150 is now a proof-and-classification card and gates nothing;
memo 019 carries the correction and the evidence.

## Operator Decision — settled 2026-08-12

**Longhorn is the update controller, for both hosts.** No Tauri updater plugin.

This reverses Card 153's correction of 2026-08-08, which recorded that "Longhorn
does not implement an installer — Tauri's updater plugin performs check,
download, verification, and bundle replacement", reshaped the gate to authorize
rather than drive, and removed the tauri dependency from the crate.

The reasoning for reversing it: Longhorn must build this reliably for GPUI
regardless, since a GPUI application has no plugin. Having a second path for
Tauri only complicates things. The bar is that Longhorn's does as well or
better than the plugin's.

### What this makes Longhorn's problem

Card 153's mechanism findings were recorded as "the guide for the
application-side wiring". They are now requirements on this crate.

**Checked 2026-08-12: three of the four below were already met on 2026-08-09**,
by the contract 018 amendment that built `longhorn-update-install`. This
section was written as though the decision opened them. It confirmed them.

- **Signature verification.** Card 153 step 6 said "never implement, wrap, or
  bypass" it, and its acceptance criterion was that verification stays entirely
  inside the plugin. Both are void. Longhorn verifies update artifacts now.
  The primitive is not new to the portfolio — `longhorn-licence` already
  verifies with `ed25519-dalek` — but the artifact path is new and it is the
  one place where "as well as the plugin" is not a matter of taste.
- **Non-writable installations.** Homebrew casks and administrator-installed
  copies need the manual-download fallback rather than an error, and the plugin
  had no typed error for it. Longhorn now owns both the detection and the
  fallback. `InstallProvenance` and `classify_install` already exist for this.
- **macOS separates install from relaunch.** The plugin handled the ordering;
  Longhorn's teardown must now interleave correctly with in-place bundle
  replacement.
- **The endpoint-only question dissolves.** Card 153's first mechanism question
  was whether Tauri installs a chosen artifact or only what its endpoint
  returns, with a loopback endpoint as the workaround. With no plugin there is
  no endpoint and no nonce.

### Tauri's updater is the design guide

Not a port and not a dependency — a reference. They have solved the problems
this crate is about to meet, and the mechanism findings Card 153 recorded came
from reading their behaviour in the first place.

Where a decision has a Tauri answer, the burden is on diverging from it rather
than on following it. Where Longhorn diverges, say why in the code.

The one place their answer is known to be short is the typed non-writable
error, recorded on Card 153 as a limitation of the app-facing surface. That is
where "as well or better" has a concrete meaning rather than a rhetorical one.

### What it makes right

Card 190's protocol was written before this decision and is more correct
because of it, not less. `UpdateCheckCommand` is Longhorn's, since Longhorn
checks. `UpdateProgressProjection` is a state the authority observes rather
than a pass-through, since Longhorn performs the download. Card 152's four
source adapters are the only check path rather than a duplicate of one.

## Execution Plan

### Batch 1. Cross-channel store compatibility

- [x] [Card 150](batch-cards/150-store-schema-stamping-and-forward-refusal.md)
  proves the existing forward-refusal end-to-end per store and gives it one
  shared classification

### Batch 2. Update policy and sources

- [x] [Card 151](batch-cards/151-update-policy-channels-and-rollout.md)
  builds `longhorn-update`: manifest model, channel resolution, semver
  comparison, client-side rollout, deferral state
- [x] [Card 152](batch-cards/152-update-source-adapters.md) defines the
  `UpdateSource` trait and the four default adapters

### Batch 3. Install and client surface

- [x] [Card 196](batch-cards/196-longhorn-is-the-update-controller.md)
  (complete 2026-08-12) builds the two pieces that did not exist: the download adapter, and the controller
  that sequences check, fetch, verify, gate and install and holds the state
  Card 190's snapshot projects
- [ ] [Card 153](batch-cards/153-restart-interlock-and-tauri-install.md)
  builds `longhorn-tauri-update`: quiescence receipt, plugin wiring, install
  — findings recorded, quiescence contract landed, and `longhorn-tauri-update`
  carries the probes and gate; the concrete installer awaits Card 159
- [x] [Card 197](batch-cards/197-cask-detection-is-backwards.md) (complete
  2026-08-13) fixes Homebrew cask detection, which Card 159's packaged run
  proved was inverted — a cask install classified as self-managed and would
  have been offered an in-place update
- [x] [Card 159](batch-cards/159-update-and-licence-packaged-proof.md)
  (update half complete 2026-08-13; the licence half belongs to g02.010)
  builds the packaged proof application Card 153's host wiring waits on,
  shared with g02.010
- [x] [Card 154](batch-cards/154-update-client-surface.md) (complete
  2026-08-13) builds the update client surface: validation, checked port,
  client, controller, and the three binding components over Poodle's
  rendering

## Dependency Shape

```text
memo 019 application update
 ├─ 150 cross-channel store proof     (independent)
 └─ 151 update policy ─┬─ 152 source adapters
                       └─ 153 restart interlock ─ 154 client surface
```

150 is independent and can run in any order. 154 consumes its shared
classification to explain a channel rejoin.

## Goals

- [x] no store loads under a schema newer than the reader understands, and
  that property is proved rather than assumed
- [x] channel selected at runtime from settings; one bundle identity
- [x] rollout decided on the client, so static hosting is sufficient
- [x] `minimum_version` and user-initiated checks both override rollout
- [x] no install proceeds while Longhorn-owned work is in flight. Card 196
      puts the gate between verify and install, so the transfer is not held
      hostage to work the replacement is.
- [x] ~~signature verification stays entirely inside the Tauri plugin~~ Void,
      and already met the other way round. The contract 018 amendment of
      2026-08-09 built `longhorn-update-install`: minisign verification, atomic
      replacement, an escalation port rather than shell interpolation,
      classified `NotWritable`, bounded extraction. The 2026-08-12 decision
      removed the plugin as a path; its job had already been taken over.
      What remains is making verification unreachable-around rather than
      promised per implementation — Card 196 step 3.
- [x] one crate and one package added, following existing naming pairs.
      `longhorn-tauri-update` was absorbed back into `longhorn-update` on
      2026-08-09 and the decision of 2026-08-12 removed the reason to recreate
      it as a plugin wrapper; a host crate for Tauri commands is still owed by
      Card 190 step 4.

## Acceptance Criteria

- [x] a store written under a newer schema is refused with a typed error,
  never partially parsed, never written back
- [x] rollout eligibility is deterministic per install and release
- [x] an install ahead of its selected channel reports that state distinctly
  from "no update available"
- [x] a refused restart defers with a reason rather than cancelling

## Explicit Non-goals

- artifact hosting, signing key custody, notarization
- delta updates, rollback, server-side rollout orchestration
- a Longhorn-published update server
- side-by-side channel installation

## Consumer Guardrail Exception

This milestone adds two crates (`longhorn-update`, `longhorn-tauri-update`
— the latter absorbed back into `longhorn-update` on 2026-08-09)
and one package (`packages/update`). The g02 remediation guardrail against
crate and package additions was scoped to remediation and does not bind new
capability work. Consumers pick the additions up when they adopt the
feature; nothing here blocks on them.

## Next Task

None. The milestone is complete.

An application can follow a channel, be offered a release, download and verify
it, refuse to install while work is in flight, install it, and come back — and
every one of those is proved rather than asserted, the last four against a real
bundle on a real machine.

**Two things leave it, neither blocking.**

`LONGHORN_PROOF_ACCEPT_LINKED_POODLE` comes out of `effigy.toml` when Poodle
publishes `SettingsShell`, `UpdateCenter` and `UpdateStatus`. Until then the
settings composition proof installs Poodle from the sibling checkout and every
run records `linkedPoodleAccepted: true`, so a green run is never mistaken for
one that proved registry resolution.

Whether `managedElsewhere` should show a quiet update icon rather than none is
with Poodle. `presence` hides it today, on the argument that an icon promises a
button that installs; the counter is that a real actionable update currently
reaches nobody who is not already in settings.

**Four of the six Tauri proofs cannot be bundled.** Only the windowing and
update proofs carry an `icons/icon.png`; `cargo check` and `clippy` pass on the
rest because `generate_context!` only demands one when bundling. So `effigy qa`
is green while four packaged proofs cannot be packaged, and their purpose is to
be run. Not this milestone's to fix, and it should not stay unrecorded.

Card 159's licence half — keychain persistence and the RFC 8252 browser flow —
belongs to g02.010 and waits on the platform `CredentialStore` decision and
Card 158.

Previously: Card 159, Card 197, Card 154, Card 190 step 4, Card 196.
