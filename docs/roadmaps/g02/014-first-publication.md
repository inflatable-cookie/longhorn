# g02.014 First Publication

Status: ready
Owner: Tom
Updated: 2026-08-11
Governing refs: contract 012; absorbed Card 166 (ready card record below)
Depends on: g02.013 complete

## Outcome

Poodle and Longhorn on the public npm registry under `@inflatable-cookie`,
every consumer on versions rather than `file:` references, and v0.1.0 tagged.

## Generation Runway

Fourteenth g02 task, and the first that is gated on something outside the
repository rather than inside it. Contract 012 held package names as "working
names until registry ownership is verified"; the `@inflatable-cookie` scope was
claimed on 2026-08-09, which satisfies that clause and opens this.

It follows g02.013 by necessity rather than by preference. Publishing before
the two consolidations would have meant deprecating twenty-one names that had
existed for exactly one release — eighteen from Longhorn, three from Poodle.

## Work

Absorbed from Card 166 (ready). Poodle Card 020 took six publish-intent
packages to three, and Longhorn Card 164 took eighteen to three; both had to
land first, and did.

### The ordering is forced

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

### Current posture, measured 2026-08-09

Everything is `private: true`, in both repositories: `poodle-core`,
`poodle-svelte`, `poodle-react`, `longhorn`, `longhorn-poodle-svelte`,
`longhorn-tauri`, all at 0.1.0. `private: true` makes `npm publish` refuse
outright, so each needs the flag dropped and `publishConfig.access: "public"`
added. **Rust is not part of this.** Every crate sets `publish = false` and
consumers take them by git tag. Repositories pinning by path today: 18 for
Poodle (26 manifests), 6 for Longhorn.

### The artifact proofs are a pre-publication device

Decided 2026-08-09: retire the Poodle half; recorded here so it is not
reopened during execution. Card 165 left `scripts/poodle-evidence.ts` deriving
the artifact-set pin from the root manifest's `file:` pack reference. Step 2
removes the pack reference it derives from, so at step 2:
`scripts/poodle-evidence.ts` is deleted, the proof consumers install Poodle
from npm by version, and every Longhorn-side composition claim is unchanged.
Contract 012's acceptance bullet "Poodle adapter evidence names the exact
preview artifact under test" becomes "names the exact published version".

### Irreversibility

npm unpublish is available for 72 hours and only while nothing depends on the
package. After that a name can be deprecated but not reclaimed or reused. This
is the step the whole "working names" clause in contract 012 was protecting,
so naming is final at step 1.

### Decisions taken 2026-08-09

**`poodle-react` is held back.** Zero consumers across the portfolio and no
preview depends on it. The name stays reserved under the claimed scope.

**One version per repository, not one version across both.** Contract 012
mandates that every Rust crate and every TypeScript package within a
repository carries the same version. Longhorn and Poodle version independently
and Longhorn depends on Poodle by ordinary semver range. Corrected 2026-08-10:
this record previously read the rule as spanning both repositories.

**Publish without a stored token.** Use npm trusted publishing (OIDC), not an
`NPM_TOKEN` secret: `permissions: id-token: write` on the job, trusted
publisher configured on npmjs.com per package before the first publish.
Provenance attestation comes with it. **No lifecycle scripts, verified.** None
of the six publish-intent packages declares `postinstall`, `prepare` or
`prepack`; it stays closed. Consumers keep installing with `--ignore-scripts`
where their tooling allows.

**Generated artifacts are committed, and gated.** Poodle Card 021 removed the
clean-clone hazard: generated icon and token trees are committed, and
`audit:icons` plus `audit:tokens` (with `--check` mode, both verified
non-vacuous) fail the release if tree and generator disagree. The release
workflow checks rather than regenerates. Verified from a clean clone on
2026-08-09.

### Steps

1. **Poodle publishes.** Drop `private`, add `publishConfig.access: "public"`,
   commit a clean tree, configure trusted publishing per package, tag, publish
   `poodle-core` and `poodle-svelte`. `poodle-react` is held.
2. **Repoint Longhorn** from `file:` packs to `^0.1.0`, deleting
   `scripts/poodle-evidence.ts` and moving the proof consumers onto the
   published version in the same change.
3. **Longhorn CI.** Done 2026-08-11: every step of the clients lane passes
   (install, 3/3 type checks, 194 package tests, 104 vitest, 1,416-file Svelte
   gate, 13/13 bindings current). The lane was never broken — the lockfile
   pinned an ephemeral Poodle pack directory that never exists on a clean
   checkout, and the lane correctly reported it for five days.
4. **Longhorn publishes** the three packages, same flag changes. Longhorn's
   three packages were found still carrying `private: true`, no
   `publishConfig`, no `license`, `files: ["src"]` — fixed after the Poodle
   dry run caught it.
5. **Tag v0.1.0**, then dispatch each release workflow against the tag ref
   (`workflow_dispatch` only; `release.yml` no longer triggers on tag push —
   Actions are dispatch-only since the 2026-08-10 org-wide disable).
6. **Consumers repoint.** 18 repositories off Poodle paths, 6 off Longhorn
   paths, then their own checks. Pure peer-satisfying `overrides` blocks go
   with them.

### Actions are dispatch-only — 2026-08-10

Every workflow in both repositories has `workflow_dispatch` as its only
trigger. The dry-run evidence: Poodle run 31387416789 green (`effigy ci`,
web-pack-install, tarballs built, publish correctly skipped); the tag path
validated by run 31391825299 dispatched against `refs/tags/v0.1.0` with
`dry-run=true`, green on all thirteen steps. Longhorn's release dry run is
green after six failed runs, each a developer-machine property a runner does
not share (sibling checkouts, cargo caches, uncoloured output, missing `rg`);
reproductions in PAPERCUTS.md.

### The first release ships against Poodle 0.2.0 — 2026-08-11

Longhorn's first published version depends on Poodle 0.2.0, not 0.1.0:
Longhorn already needs unreleased Poodle (`7827bd7d` forwards a DockRegion
`showTabs` prop missing from 0.1.0). CI cannot run until Poodle 0.2.0
publishes, and that is accepted — develop against a linked local Poodle with
the manifest unchanged. The one open failure was Poodle's Dialog focus-restore
leak, resolved upstream (`3889825e`) with bounded, cancellable restore
semantics now stated in Poodle's contract; Longhorn needed no change.

## Disclosure readiness

Card 167 (complete 2026-08-09) removes third-party identity so either
repository can be made public. Independent of publication — publication does
not require public repositories, but the operator intends them.

## Goals

- the six publish-intent packages resolve from the public registry
- no repository in the portfolio pins Poodle or Longhorn by path
- Longhorn's CI TypeScript lane completes for the first time (done 2026-08-11)
- v0.1.0 tagged, Rust by git tag and TypeScript by version
- no `NPM_TOKEN` secret exists in either repository, at any point
- every workflow in both repositories triggers only on `workflow_dispatch`
- trusted publishing configured for all five published packages
- a tarball built from a clean checkout contains the generated icon and token trees
- contract 012's "working names" and "publication is deferred" clauses updated
  to describe what actually happened

## Non-goals

- **crates.io.** Every Rust crate sets `publish = false` and consumers take
  them by git tag.
- **Card 149's candidate receipt.** It freezes a cross-repository consumer
  graph and stays operator-held on manifest quiescence, which publication does
  not provide.

## Risks

The step is close to irreversible. Naming is final at first publish, which is
precisely what contract 012's working-names clause was protecting. Poodle's
release job moved from `macos-latest` to `ubuntu-latest`; Longhorn's stays on
macOS where the gpui dependency makes it genuine.

## Absorbed records

- Card 166: ready — scope, ordering, posture, decisions, steps, acceptance, and evidence absorbed inline above.
- Card 167: complete 2026-08-09 — publication disclosure readiness.

## Evidence required

- the published versions and their integrity hashes
- a clean-clone install of one consumer with no `file:` reference anywhere in its lockfile
- the first green CI run of the clients lane (done 2026-08-11)
