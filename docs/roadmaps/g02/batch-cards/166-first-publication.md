# 166 First Publication

Status: ready
Owner: Tom
Roadmap: g02.014 batch 1
Governing refs: contract 012 (Workspace And Versions, Consumer Adoption)
Depends on: Card 164 complete; Card 165 complete; Poodle Cards 020 and 021
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

**Decided 2026-08-09: retire the Poodle half.** Recorded here so it is not
reopened during execution.

Card 165 left `scripts/poodle-evidence.ts` deriving the artifact-set pin from
the root manifest's `file:` pack reference. That is correct while Poodle is
consumed as local tarballs and strictly better than the eight hardcoded paths
it replaced. Step 2 removes the pack reference it derives from.

The Poodle evidence file — set id, per-tarball digests, `svelteFloor` — exists
for exactly one reason: Poodle is unpublished. With no registry, naming an
exact Poodle build requires a local directory and a set of hashes. That is the
only way to say *this* artifact.

A published version says it natively, and better: the lockfile pins the
version and npm records an integrity hash the registry enforces. Keeping the
evidence file after publication would be an elaborate restatement of what
`"@inflatable-cookie/poodle-core": "0.1.0"` already pins, and it would need a
new source of truth invented for it.

So at step 2:

- `scripts/poodle-evidence.ts` is deleted, and with it the derivation problem
  — nothing needs a pack path once no pack path exists
- the isolated proof consumers install Poodle from npm by version
- **every Longhorn-side composition claim is unchanged** — which packages
  resolve, import-absence, one Svelte runtime, no workspace references in the
  lockfile, mounted component tests

Contract 012's acceptance bullet "Poodle adapter evidence names the exact
preview artifact under test" becomes "names the exact published version". A
wording amendment, not a weakened claim.

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

## Generated Artifacts Are Committed, And Gated — Resolved 2026-08-09

This card previously recorded a hazard: `poodle-core`'s generated icon and
token trees were `.gitignore`d, present in a packed tarball only because they
existed on the machine that packed it. A clean checkout had neither, so
publishing from CI would have shipped an empty `icons/` directory with nothing
failing.

**Poodle Card 021 removed the hazard rather than automating around it.** The
generated artifacts are now committed — 100 icon modules and 25 token files —
so a clean clone packs a complete tarball with no build step and
`lucide-static` is not needed at pack time at all.

Committing generated source is only safe when something fails if the tree and
the generator disagree, and both halves now have that. `audit:icons` was
already the first step of `ci:web`; `audit:tokens` was added alongside it on
2026-08-09, after the token generator gained a `--check` mode. Both were
verified non-vacuous by perturbing a generated file and confirming the gate
fails — the same trap as an absence assertion that can only ever pass.

The release workflow **checks rather than regenerates**, deliberately.
Regenerating before packing would ship whatever the generator produced on the
runner, which can differ from what was reviewed and tagged. Checking fails the
release instead, and the tarball is exactly the reviewed tree.

Verified from a clean clone on 2026-08-09: `effigy ci` exits 0, the
packed-consumer proof passes, and `poodle-core` packs to 180,208 bytes
carrying 100 icon modules, 22 token CSS files, LICENSE and README, with no
runtime dependencies.

## Steps

1. **Poodle publishes.** Drop `private`, add `publishConfig.access: "public"`,
   commit a clean tree, configure trusted publishing per package, tag, publish
   `poodle-core` and `poodle-svelte`. `poodle-react` is held. Generation is no
   longer part of this — the artifacts are committed and gated.
2. **Repoint Longhorn** from `file:` packs to `^0.1.0`, deleting
   `scripts/poodle-evidence.ts` and moving the proof consumers onto the
   published version in the same change.
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
- `proof:artifacts` green with Poodle installed from the registry
- v0.1.0 tagged, with Rust by tag and TypeScript by version
- contract 012's "working names" and "publication is deferred" clauses updated
  to describe what actually happened
- no `NPM_TOKEN` secret exists in either repository, at any point
- trusted publishing is configured for all five published packages
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
