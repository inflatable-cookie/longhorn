# 218 Linked-poodle Exit Gate

Status: operator-held on Poodle v0.2.0 — held whole; no machinery has landed
Held because: Poodle v0.2.0 is in active development and Longhorn's release
depends on functionality arriving in it. The operator's decision of 2026-08-15
is that Longhorn's release waits for Poodle rather than gating against the
0.1.0 peer, so the exemption stays and the assertion machinery is deferred
with the rest of the card. Recorded rather than assumed: an earlier revision
of this line read "machinery lands now", which was never true of the tree.
Owner: Tom
Roadmap: g02.026 batch 1
Governing refs: contract 012; memo 023 (C1)
Depends on: none to start; final criterion requires the published
`@inflatable-cookie/poodle-svelte` to contain the components
`longhorn-poodle-svelte` imports (Poodle v0.2.0, operator-named precondition)
Auto-start next card: no

## Objective

The linked-Poodle exemption that keeps local development possible cannot pass
a release unnoticed — and dies by gate the moment Poodle v0.2.0 publishes.

## Why this exists

Local gates type-check against a bun global link into the sibling Poodle
checkout (`node_modules/@inflatable-cookie/poodle-svelte` →
`~/.bun/install/global/...`), while the published `poodle-svelte@0.1.0`
tarball — downloaded and listed — contains none of the `SettingsShell` /
`UpdateCenter` / `UpdateStatus` components the package's public exports
import. `LONGHORN_PROOF_ACCEPT_LINKED_POODLE=1` is hardwired into
`proof:artifacts` (`effigy.toml:68`, marked "REMOVE THIS"), `linkedPoodle
Accepted: true` is recorded, and nothing fails a release on it. The operator
has named the resolution: Longhorn's release waits on Poodle v0.2.0. This
card builds the assertion machinery so that wait is enforced, not remembered.

## Scope

- `scripts/poodle-release.ts` — integrity verification
- `effigy.toml` / `config/release.toml` — the release-gate assertion
- `scripts/ci-rehearse.sh` — bun link-state awareness
- a pack-level typecheck against registry Poodle

## Steps

1. Release gates fail when `linkedPoodleAccepted` is true. The exemption stays
   available for local proofs; the release path asserts it is off.
2. `poodle-release.ts` verifies what its header claims — the integrity bytes
   of the installed tree against the registry tarball, not the version string
   a linked checkout also reports.
3. `ci-rehearse.sh` resets or detects bun global-link state, the way it
   already resets `CARGO_HOME` — the rehearsal must see what a clean runner
   sees.
4. Pack-level typecheck: `bun pm pack` `longhorn-poodle-svelte`, install into
   a scratch dir with *registry* Poodle, typecheck the public surfaces. This
   fails today against 0.1.0 — that failure is the point; the gate documents
   the Poodle v0.2.0 precondition.
5. When Poodle v0.2.0 publishes: remove the exemption from `proof:artifacts`,
   delete the "REMOVE THIS" comment, close the card.

## Do Not

- Remove the local exemption before v0.2.0 — it is what keeps local evidence
  possible, and the operator has named the timeline.
- Assert against the installed tree. The registry tarball is the truth.

## Acceptance Criteria

- [ ] `effigy release gates` fails if run with a linked Poodle satisfying the
  peer
- [ ] `ci:rehearse` observes or resets bun global links
- [ ] the pack-level typecheck exists and currently fails against 0.1.0, with
  the precondition recorded
- [ ] (held) after Poodle v0.2.0: the exemption is removed and the typecheck
  passes against the registry

## Evidence Required

- the gate failing under a link and passing unlinked
- the rehearsal output showing link-state detection

## Stop Conditions

None anticipated; the held criterion is explicit.
