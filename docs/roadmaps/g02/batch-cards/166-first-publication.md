# 166 First Publication

Status: ready
Owner: Tom
Roadmap: g02.014 batch 1
Governing refs: contract 012 (Workspace And Versions, Consumer Adoption)
Depends on: Card 164 complete; Card 165 complete; Poodle Card 020 complete
Auto-start next card: no

## Objective

Publish Poodle and Longhorn to npm under `@inflatable-cookie`, move every
consumer off `file:` references onto versions, and cut the v0.1.0 tag.

## What Unblocked This

Contract 012 holds that "package names are working names until registry
ownership is verified before first publication". The `@inflatable-cookie`
scope was claimed on 2026-08-09, so the names are now ownable and the gate is
satisfied.

Two other things had to land first, and did. Poodle Card 020 took six
publish-intent packages to three, and Longhorn Card 164 took eighteen to
three. Publishing before either would have meant deprecating twenty-one names
that existed for one release.

## The Ordering Is Forced

Longhorn depends on Poodle, and eighteen repositories depend on Poodle
directly. Nothing downstream can move to versions until the thing it depends
on is on the registry.

```
1. Poodle publishes           poodle-core, poodle-svelte
2. Longhorn repoints          file: packs -> ^0.1.0
3. Longhorn CI runs           the clients lane, for the first time
4. Longhorn publishes         longhorn, -poodle-svelte, -tauri
5. Longhorn tags v0.1.0       Rust by git tag; TypeScript by version
6. Consumers repoint          18 repos off Poodle paths, 6 off Longhorn paths
```

Steps 2 and 6 are sweeps this session has run three times; the mechanics are
known and the specifier maps are correct. Step 3 is the one that has never
been exercised.

## Current Posture, Measured 2026-08-09

Everything is `private: true`, in both repositories:

| Package | private | version |
| --- | --- | --- |
| `@inflatable-cookie/poodle-core` | yes | 0.1.0 |
| `@inflatable-cookie/poodle-svelte` | yes | 0.1.0 |
| `@inflatable-cookie/poodle-react` | yes | 0.1.0 |
| `@inflatable-cookie/longhorn` | yes | 0.1.0 |
| `@inflatable-cookie/longhorn-poodle-svelte` | yes | 0.1.0 |
| `@inflatable-cookie/longhorn-tauri` | yes | 0.1.0 |

`private: true` makes `npm publish` refuse outright, so each needs the flag
dropped and `publishConfig.access: "public"` added — a scoped package defaults
to restricted, and restricted on a free account fails.

**Rust is not part of this.** Every crate sets `publish = false` and consumers
take them by git tag. The Rust half of v0.1.0 needs only the tag, which is why
"tag Poodle" was an incomplete description of the runway.

Repositories pinning by path today: **18 for Poodle** (26 manifests), **6 for
Longhorn**.

## The Artifact Proofs Are A Pre-publication Device

Card 165 left `scripts/poodle-evidence.ts` deriving the artifact-set pin from
the root manifest's `file:` pack reference. That is correct while Poodle is
consumed as local tarballs and strictly better than the eight hardcoded paths
it replaced. **Step 2 removes the pack reference it derives from.**

This is not a defect to patch around. The whole artifact-proof model — pack a
sibling build directory, install it into an isolated root, assert the graph —
exists to prove something that publication proves better: that a consumer can
install the real thing and compose it. After step 2 the proofs should verify
the *published* tarball, which is a smaller and truer claim.

Decide this before step 2, not after. The options:

- **Point the proofs at the published tarball.** `npm pack @inflatable-cookie/
  poodle-core@0.1.0` gives the same artifact shape the proofs already handle,
  and the digest claim becomes meaningful rather than self-referential.
- **Retire the Poodle half of the proofs** and keep only the Longhorn-side
  composition claims. The Poodle evidence file exists because Poodle was
  unpublished; a version range is the ordinary way to express the same thing.

The first keeps the evidence, the second admits publication replaced the need
for it. Either is defensible; drifting into neither is not.

## Irreversibility

npm unpublish is available for 72 hours and only while nothing depends on the
package. After that a name can be deprecated but not reclaimed or reused. This
is the step the whole "working names" clause in contract 012 was protecting,
so the naming should be treated as final at step 1.

## Decisions Taken 2026-08-09

**`poodle-react` is held back.** Zero consumers across the portfolio and no
preview depends on it. The name stays reserved under the claimed scope, so
holding costs nothing and publishing would start a support surface for
something unexercised.

**One version across everything.** Every Rust crate and every TypeScript
package moves in lockstep, in both repositories — contract 012 already
mandates this within Longhorn, and it now spans Poodle too. When
`poodle-react` does ship, it ships at whatever the common version is, not at
`0.1.0` from a standing start.

## Publish Without A Stored Token

The recent npm worms propagated through stolen long-lived automation tokens
and install-time lifecycle scripts. Both are avoidable here rather than
mitigated.

**Use npm trusted publishing (OIDC), not an `NPM_TOKEN` secret.** npm can be
configured to accept publishes for a package only from a named GitHub
repository and workflow file, authenticating with a short-lived OIDC token
minted per run. Nothing long-lived is stored in GitHub, so there is no secret
for a compromised dependency to exfiltrate and no token to rotate. It needs
`permissions: id-token: write` on the job, a recent npm CLI, and the trusted
publisher configured on npmjs.com per package before the first publish.
Provenance attestation comes with it rather than being a separate step.

If a token is used instead, it should be granular, scoped to these packages
only, and short-expiry — but the point of trusted publishing is that the
question does not arise.

**No lifecycle scripts, verified.** None of the six publish-intent packages
declares `postinstall`, `prepare` or `prepack`; `poodle-core`'s `test` and
`check` are not install hooks. That is the other half of the worm vector and
it is already closed. It should stay closed — a `prepack` added later for
convenience would reintroduce it.

Consumers should keep installing with `--ignore-scripts` where their tooling
allows, which the artifact proofs already do.

## A Clean Clone Does Not Currently Publish Correctly

`poodle-core`'s 1,703 icon modules and 25 generated token files are
`.gitignore`d and produced by `packages/core/src/icons/generate.mjs` and the
token build. They are in the packed tarball today only because they exist on
the machine that packed it.

**A fresh checkout has neither.** Publishing from CI — which is what trusted
publishing means — would ship `poodle-core` with an empty `icons/` directory
and no token CSS, and nothing in the current setup would fail. `bun pm pack`
does not run a generate step, and the pack-install proof passes because it
runs on a warm working tree.

So step 1 needs the generation wired into the release workflow before the
pack, and a check that the tarball contains the icon tree rather than trusting
that it does. This is the sharpest reason not to publish from a laptop: the
laptop is the only place that currently produces a correct artifact, and for
the wrong reason.

## Steps

1. **Poodle publishes.** Drop `private`, add `publishConfig.access: "public"`,
   wire icon and token generation into the release workflow ahead of the pack,
   assert the tarball carries both, configure trusted publishing per package,
   tag, publish `poodle-core` and `poodle-svelte`. `poodle-react` is held.
2. **Decide the proof model** per the section above, then repoint Longhorn's
   root manifest and overrides from `file:` packs to `^0.1.0`.
3. **Longhorn CI.** The clients lane has never completed — `bun install
   --frozen-lockfile` has failed on the machine-local Poodle path since
   2026-08-06. This is the first honest run of the TypeScript lane.
4. **Longhorn publishes** the three packages, same flag changes.
5. **Tag v0.1.0.** Rust consumers take crates by git tag; TypeScript consumers
   take packages by version. `effigy release:gates` runs the floor and
   source-consumer checks.
6. **Consumers repoint.** 18 repositories off Poodle paths, 6 off Longhorn
   paths, then their own checks. The `overrides` blocks that exist purely to
   satisfy peers under `file:` references can be deleted at the same time —
   that papercut closes on publication.

## Acceptance Criteria

- `@inflatable-cookie/poodle-core`, `-svelte`, `@inflatable-cookie/longhorn`,
  `-poodle-svelte`, `-tauri` resolve from the public registry
- no repository in the portfolio pins Poodle or Longhorn by `file:`
- Longhorn's CI clients lane completes on a clean checkout
- `proof:artifacts` green under whichever proof model step 2 chooses
- v0.1.0 tagged, with Rust by tag and TypeScript by version
- contract 012's "working names" and "publication is deferred" clauses updated
  to describe what actually happened
- no `NPM_TOKEN` secret exists in either repository
- a tarball built from a clean checkout contains the generated icon and token
  trees

## Evidence Required

- the published versions and their integrity hashes
- a clean-clone install of one consumer with no `file:` reference anywhere in
  its lockfile
- the first green CI run of the clients lane

## Notes

Card 149's candidate receipt is the one thing this does not unblock. It freezes
a cross-repository consumer graph and stays operator-held on manifest
quiescence, which publication does not provide.
