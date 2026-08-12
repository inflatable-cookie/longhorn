# g02.009 Application Update And Release Channels

Status: ready
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

- [ ] [Card 196](batch-cards/196-longhorn-is-the-update-controller.md) builds
  what the operator decision made Longhorn's: artifact verification, the
  download, and the controller that sequences check, fetch, verify, gate and
  install
- [ ] [Card 153](batch-cards/153-restart-interlock-and-tauri-install.md)
  builds `longhorn-tauri-update`: quiescence receipt, plugin wiring, install
  — findings recorded, quiescence contract landed, and `longhorn-tauri-update`
  carries the probes and gate; the concrete installer awaits Card 159
- [ ] [Card 159](batch-cards/159-update-and-licence-packaged-proof.md)
  builds the packaged proof application Card 153's host wiring waits on,
  shared with g02.010
- [ ] [Card 154](batch-cards/154-update-client-surface.md) builds
  `packages/update`: bindings and Svelte surface

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
- [ ] no install proceeds while Longhorn-owned work is in flight
- [ ] ~~signature verification stays entirely inside the Tauri plugin~~ Void
      from 2026-08-12. Longhorn verifies, and the goal is now that no
      unverified artifact can reach an installer by the shape of the call
      rather than by each implementation's promise. Card 196.
- [ ] one crate and one package added, following existing naming pairs.
      `longhorn-tauri-update` was absorbed back into `longhorn-update` on
      2026-08-09 and the decision of 2026-08-12 removed the reason to recreate
      it as a plugin wrapper; a host crate for Tauri commands is still owed by
      Card 190 step 4.

## Acceptance Criteria

- [ ] a store written under a newer schema is refused with a typed error,
  never partially parsed, never written back
- [ ] rollout eligibility is deterministic per install and release
- [ ] an install ahead of its selected channel reports that state distinctly
  from "no update available"
- [ ] a refused restart defers with a reason rather than cancelling

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

Card 196, the controller. Card 190 landed the protocol on 2026-08-12 and
stopped at step 4 for want of a host crate; Card 154 is blocked behind it; Card
159 is paused by operator decision. The controller is the only unblocked work
in the batch, and it is what the decision of 2026-08-12 made Longhorn's without
anything then being written down that builds it.

Two corrections are recorded here rather than silently applied. This section
said "Open Card 150" while 150 to 153 were all complete. And the goal
"signature verification stays entirely inside the Tauri plugin" survived the
decision that voided it by nine days.
