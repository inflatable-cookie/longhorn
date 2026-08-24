# 218 Linked-poodle Exit Gate

Status: needs recompile — step 5 landed 2026-08-24 (g16.008); step 1 is
obsolete; steps 2-4 have not landed
Unheld because: the operator-named precondition is satisfied. It was never the
v0.2.0 number, it was the components — published
`@inflatable-cookie/poodle-svelte` had to contain what `longhorn-poodle-svelte`
imports. Public 0.2.2 contains `SettingsShell`, `UpdateCenter` and
`UpdateStatus` and exports all three, so the wait ended at 0.2.2 rather than
at 0.2.0. (0.2.0 and 0.2.1 are not adoptable: 0.2.1 exposed a fork-sourced
GPUI identity. See Poodle g16.005 and g16.007.)
Owner: Tom
Roadmap: g02.026 batch 1
Governing refs: contract 012; memo 023 (C1)
Depends on: nothing outstanding — the Poodle precondition is met
Auto-start next card: no

## Objective

The linked-Poodle exemption that keeps local development possible cannot pass
a release unnoticed — and dies the moment Poodle publishes the components the
adapter imports.

It died by deletion rather than by gate. The card planned to build the
assertion machinery first so the exemption would fail a release the moment it
became unnecessary; in the event the release arrived before any of that
machinery, so g16.008 removed the exemption outright. What remains of this
card is the release hardening that was never only about the exemption.

## Why this existed

Recorded as of 2026-08-15, when the card was written. Half of it is now
history; the bun-link half is not.

Local gates type-checked against a bun global link into the sibling Poodle
checkout (`node_modules/@inflatable-cookie/poodle-svelte` →
`~/.bun/install/global/...`), while the published `poodle-svelte@0.1.0`
tarball — downloaded and listed — contained none of the `SettingsShell` /
`UpdateCenter` / `UpdateStatus` components the package's public exports
import. `LONGHORN_PROOF_ACCEPT_LINKED_POODLE=1` was hardwired into
`proof:artifacts`, marked "REMOVE THIS", `linkedPoodleAccepted: true` was
recorded, and nothing failed a release on it. The operator named the
resolution: Longhorn's release waits on Poodle rather than gating against
the 0.1.0 peer. This card was to build the assertion machinery so that wait
was enforced, not remembered.

The wait is over — public 0.2.2 ships the components, and g16.008 deleted the
exemption. A bun global link can still make a local gate pass against bytes no
consumer resolves, which is what steps 2-4 remain for.

## Scope

- `scripts/poodle-release.ts` — integrity verification
- `scripts/ci-rehearse.sh` — bun link-state awareness
- a pack-level typecheck against registry Poodle

## Steps

1. ~~Release gates fail when `linkedPoodleAccepted` is true.~~ **Obsolete
   2026-08-24.** g16.008 deleted the exemption and the
   `linkedPoodleAccepted` field with it, so there is no flag for a gate to
   assert is off. A gate on a field nothing writes is not a gate.
2. `poodle-release.ts` verifies what its header claims — the integrity bytes
   of the installed tree against the registry tarball, not the version string
   a linked checkout also reports.
3. `ci-rehearse.sh` resets or detects bun global-link state, the way it
   already resets `CARGO_HOME` — the rehearsal must see what a clean runner
   sees.
4. Pack-level typecheck: `bun pm pack` `longhorn-poodle-svelte`, install into
   a scratch dir with *registry* Poodle, typecheck the public surfaces. This
   was written to fail against 0.1.0, and that failure was the point. Against
   registry 0.2.2 it should pass, so it becomes an ordinary gate rather than a
   documented precondition.
5. ~~When Poodle publishes: remove the exemption from `proof:artifacts`,
   delete the "REMOVE THIS" comment.~~ **Landed 2026-08-24** in g16.008
   (Longhorn PR 9). The forced `LONGHORN_PROOF_ACCEPT_LINKED_POODLE=1`, its
   `effigy.toml` comment, `ACCEPT_LINKED_POODLE`, `packLinkedPoodle` and the
   `linkedPoodleAccepted` artifact field are gone. Every proof resolves
   registry Poodle 0.2.2 through `poodleRelease()`. The card does not close
   with it: steps 2-4 are still open.

## Do Not

- Reintroduce the exemption. It is gone, and the condition that justified it
  cannot recur without a Poodle regression.
- Assert against the installed tree. The registry tarball is the truth.

## Acceptance Criteria

- [x] the exemption is out of `proof:artifacts`, and no code path can set it
      (2026-08-24, g16.008)
- [x] the published `@inflatable-cookie/poodle-svelte` contains the components
      `longhorn-poodle-svelte` imports — 0.2.2 ships and exports all three
- [ ] ~~`effigy release gates` fails if run with a linked Poodle satisfying
      the peer~~ — obsolete with step 1: there is no flag left to assert on
- [ ] `poodle-release.ts` verifies the registry tarball's integrity bytes, not
      the version string a linked checkout also reports
- [ ] `ci:rehearse` observes or resets bun global links
- [ ] the pack-level typecheck exists and passes against registry Poodle

## Evidence Required

- the rehearsal output showing link-state detection
- the pack-level typecheck passing against a registry install

## Recompile Note

Steps 2-4 were scoped as the exemption's exit machinery, and the exemption has
exited without them. They are still worth having — a local bun global link can
still make a gate pass against bytes no consumer resolves — but the card's
framing no longer matches its remaining work, and step 1 is now a gate with
nothing to gate on. Recompile the remainder as release hardening, or fold it
into whichever release lane owns the runner environment, before treating any
of it as ready.

## Stop Conditions

None anticipated.
