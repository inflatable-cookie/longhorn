# g02.039 Poodle 0.4.2 Adoption

Owner: Tom
Created: 2026-09-14
Governing refs: contracts 012 and 013; Poodle CHANGELOG 0.4.0/0.4.2 at tag
`v0.4.2`; g16.109 precedent (the 0.3.0 adoption lane)
Depends on: no other task. Independent of g02.036 L1 dispatch and of g02.008.
UI classification: none

## Outcome

Every Longhorn Poodle pin moves to published 0.4.2: npm exact `0.4.2` for
`@inflatable-cookie/poodle-core` and `@inflatable-cookie/poodle-svelte`, git
tag `v0.4.2` for `poodle-specs` and both GPUI prototypes. One identity end to
end — root manifest, adapter peer, example pins, boundary test, locks, and
proof machinery agree. Contracts 012/013 checkpoints name the new pin.

## Verified publish and retriage facts (2026-09-14)

- Operator retriage amended the target from 0.4.1 to the immutable public
  replacement 0.4.2 after the original latest-moved stop condition fired.
- npm `latest` is `0.4.2` for both packages. `poodle-svelte` peers: svelte
  `>=5.56.8 <6` (the root pins exactly `5.56.8`) and `marked` `^18.0.9`,
  still optional. `poodle-core@0.4.2` keeps the `poodle-icons` bin and the
  `.`, `./styles/*`, `./tokens/*`, `./icons/*` subpath families.
- Tag `v0.4.2` dereferences to
  `d2438aef7d34df31b958d172c9c162e83063d83c`. Every consumed
  crate on that tag is version `0.4.0` — Cargo packages do not move in a
  patch release. So `poodle-specs` becomes
  `{ version = "0.4.0", tag = "v0.4.2" }`, not `"0.4.2"`.
- Hosted candidate run `34838863944` passed. Publish run `34838995758`
  published both archives and timed out only while waiting for delayed Svelte
  registry visibility. Both public npm tarballs now exist and byte-match the
  certified SHA-256 values: core
  `bbdc9110eca86d6b9ff75cfd55ce5ed50d060225c351c08230e950d27cb5eb45`,
  Svelte
  `a2210e6d95b5a6b5ec858b0cc32d7ebd65466eb600336267976d6789127d0bb1`.
- `cargo 1.97.1` resolves the tag (probe-locked all seven consumed crates at
  0.4.0). The tag has no root `Cargo.toml` — npm-candidate isolation moved
  it — but git dependency resolution still works. `gpui` stays crates.io
  `0.2.2`: the contract 012 one-source invariant holds. `zvariant` and kin
  arrive transitively.

## Breaking-change exposure (0.3.0 → 0.4.2)

Poodle 0.4.0 removed or re-broke: Slider/RangeSlider `appearance`→`variant`
plus eighteen recipe hooks; markdown sanitize-by-default (`renderHtml` no
longer implies trust); `poodle-specs` Slider* items; `poodle-headless` text
helpers; `poodle-node` gained `NodeRole` variants and `NodeA11y.initial_focus`
(exhaustive matches break); `poodle-render` slider entry points changed
signature.

Longhorn consumes none of these. Verified by enumerating every use site:

- `poodle_specs`: `BannerSpec`, `StatusTone`, `ProgressSpec`,
  `StatusIndicatorSpec`, `ChoiceOption`, `DetailItemSpec`, `RadioGroupSpec`,
  `SidebarNavGroup/Item/Spec`.
- `poodle_render`: `toast_stack`, `banner`, `progress`, `status_indicator`,
  `RenderContext`. `poodle_adapter`: `ThemeProvider`. `poodle_gpui`:
  `GpuiThemeProvider`. `poodle_gpui_node_backend`: `reset_element_ids`,
  `color`, `to_gpui`.
- TypeScript imports are root-namespace only — Layout, settings,
  notifications, licence, update, operation, and commands components. No
  Slider, no `renderHtml`, no `htmlPolicy` anywhere.

Expected code migration: zero. Any fallout that does surface is bounded
compatibility fallout absorbed in this lane. No shims, aliases, or fallbacks.

Poodle 0.4.2 exposes the public `TabsItem` cross-window bridge forwarding
required by Longhorn's prepare oracle. A fresh public-registry consumer passed
`DockRegion.test.ts` 4/4, including that cross-window bridge case. This is the
producer exposure being adopted; Longhorn adds no adapter shim or private
source path.

## Work

1. Root `package.json` devDeps → exact `0.4.2`, both packages.
2. `packages/longhorn-poodle-svelte` peerDependencies → exact `0.4.2`, both
   still optional; `tests/boundary.test.ts` expected peer versions → `0.4.2`.
3. All seventeen example consumer `package.json` Poodle pins → `0.4.2`, in
   lockstep with the adapter peer so one identity holds.
4. `crates/longhorn-poodle/Cargo.toml` `poodle-specs` → version `"0.4.0"`,
   tag `"v0.4.2"`; both GPUI prototype manifests' four Poodle git tags →
   `v0.4.2` (same-source invariant).
5. Regenerate locks, Poodle-limited: `bun.lock`, root `Cargo.lock`, and both
   prototype `Cargo.lock`s. No unrelated churn.
6. Contract 012 "Current Poodle Checkpoint" and the contract 013
   compatibility bullet name exact `0.4.2` / tag `v0.4.2` at `d2438aef…`,
   adopted by g02.039.
7. `docs/reference/api-surface.md` adapter row Poodle peers → `0.4.2`.
8. `scripts/poodle-release.ts` header prose (the "0.3.0 as of g16.109" line).
9. Run the proof machinery against the new pin: release integrity check,
   greenfield card125 verifier, artifact proofs, adapter boundary test,
   prototype builds.

## Acceptance criteria

- One Poodle identity on each side: `bun.lock` resolves exactly one
  `poodle-core`/`poodle-svelte` at `0.4.2`; every `Cargo.lock` pins tag
  `v0.4.2` at `d2438aef7d34df31b958d172c9c162e83063d83c`.
- A pin sweep finds no live `0.3.0`/`v0.3.0` Poodle remnant outside
  historical records (superseded contract checkpoints, old handoffs, logs,
  the card127 receipt fixture).
- Adapter boundary test green; narrow Poodle checks and full `effigy qa`
  pass; `git diff --check` clean.
- Public 0.4.2 exposure is recorded: certified npm tarball hashes and a fresh
  consumer passes `DockRegion.test.ts` 4/4, including the bridge prepare oracle.

## Evidence required

- Lock identity greps (Bun and Cargo) in the PR description, plus the 0.4.2
  exposure statement and release evidence.
- `effigy qa` receipt.
- One reviewable PR; merge stays with the orchestrator.

## Stop conditions

- Public registry resolution fails or `latest` moves off `0.4.2`.
- The bump exposes a Longhorn defect that is not compatibility fallout.
- Lock churn reaches unrelated packages.
- Fallout would need an API shim, alias, or silent fallback to absorb — the
  pre-1.0 posture forbids it. Stop and retriage instead.

## Explicit non-goals

- g02.008 candidate receipt generation. This adoption answers its step-1
  precondition — the npm set is settled at core+svelte, 0.4.2 — but the
  receipt lane stays its own task.
- Consumer repository edits. The five consumer apps adopt 0.4.2 in their own
  operator-driven wave (the g16.109 pattern). The adapter peer move is the
  coordination point, not authorization to write consumer trees.
- Longhorn version bump or publication.
- Desktop/native/windowed proofs.
- The adapter's svelte peer range. Its `>=5.38.6` floor already sits below
  Poodle's `>=5.56.8` floor; that mismatch predates this lane. Do not widen
  or retune it here; record it if it bites the boundary test.

## Downstream coordination

- Consumers installing `@inflatable-cookie/longhorn-poodle-svelte` must move
  their own Poodle pins to `0.4.2` when they take the adapter peer move, and
  must be on svelte `>=5.56.8` (the 0.4.2 peer floor).
- After this lands, g02.008 may resume receipt generation against the
  settled artifact set.

## Next Task

g02.008 receipt generation, once its coordinator picks it up with the 0.4.2
pin in place.
