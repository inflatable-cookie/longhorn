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

**One version per repository, not one version across both.** Contract 012
mandates that every Rust crate and every TypeScript package within a
repository carries the same version, so a tag names all of them at once. That
is where it stops. Longhorn and Poodle version independently and Longhorn
depends on Poodle by ordinary semver range.

**Corrected 2026-08-10.** This card previously read the rule as spanning both
repositories. It does not, and the difference is not cosmetic: cross-repository
lockstep would have forced Longhorn to wait for a Poodle release it does not
need, and would have coupled two release cadences that have no reason to move
together. When `poodle-react` does ship, it ships at Poodle's common version.

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
3. **Longhorn CI.** ~~The clients lane has never completed.~~ Done: every step
   passes as of 2026-08-11, once the repoint removed the ephemeral pack path
   the lockfile pinned. See below.
4. **Longhorn publishes** the three packages, same flag changes.
5. **Tag v0.1.0**, then dispatch each release workflow against the tag ref.
   Rust consumers take crates by git tag; TypeScript consumers take packages
   by version. `effigy release:gates` runs the floor and source-consumer
   checks.
6. **Consumers repoint.** 18 repositories off Poodle paths, 6 off Longhorn
   paths, then their own checks. The `overrides` blocks that exist purely to
   satisfy peers under `file:` references can be deleted at the same time —
   that papercut closes on publication.

## Actions Are Dispatch-Only — 2026-08-10

Actions were disabled org-wide after the allowance ran out, and they are
re-enabled per repository once its workflows are known safe. Every workflow in
both repositories now has `workflow_dispatch` as its only trigger: no push, no
pull_request, no schedule.

The cost was concentrated in macOS. Poodle ran a nightly `ci-native` job behind
a "did the default branch move in the last 24h" guard, which is true every day
during active development, so it was a daily macOS run at ten times the Linux
rate. Longhorn runs *all four* CI jobs on macOS, because `effigy qa` builds
`longhorn-gpui-windowing` and gpui pulls metal.

The failure mode is worth recording because it does not look like a budget
problem. Dispatched runs sat queued indefinitely, and both `cancel` and
`force-cancel` returned HTTP 500 while githubstatus.com reported all systems
operational. That reads like a GitHub incident.

This changes step 5. `release.yml` no longer has a `push: tags` trigger in
either repository; dispatch it against the tag ref instead, which
`workflow_dispatch` accepts, so `github.ref` is `refs/tags/v0.1.0` and the
version-agrees-with-the-tag check still fires:

```sh
git tag v0.1.0 && git push origin v0.1.0
gh workflow run release.yml --ref v0.1.0 -f dry-run=false
```

That ordering matters here specifically. The bootstrap below publishes by hand
*before* trusted publishers exist, so a tag push would fire a run attempting to
republish versions already on the registry — a red run at the exact moment the
release needs to look trustworthy.

Poodle's release job also moved from `macos-latest` to `ubuntu-latest`.
`effigy ci` is `ci:web` plus `ci:rust`; `ci:native` is the only macOS-bound
lane and is deliberately not part of it. Longhorn's stays on macOS, where the
gpui dependency makes it genuine.

## Poodle Dry Run Green — 2026-08-10

Run 31387416789. `setup-effigy` installs 0.9.1, `effigy ci` passes in CI,
`effigy test:web-pack-install` passes, the tarballs build and the publish step
correctly skips.

| | core | svelte |
| --- | --- | --- |
| bytes | 180,217 | 192,552 |
| entries | 340 | 175 |
| icon modules | 100 | — |
| token stylesheets | 22 | — |

That table is the answer to this card's clean-clone hazard. The worry was a
release workflow packing an empty `icons/` because the generator had not run;
the icons and tokens are committed, `effigy ci` proves they are not stale, and
these counts prove `files` carried them into a tarball built from a clean
checkout. Nothing regenerates during the release.

Two defects the dry run found, both mine:

**`tar -tzf | head -20` failed on ubuntu.** GNU tar treats the EPIPE as an
error and `set -o pipefail` fails the step; BSD tar does not, so it passed on
macOS and broke on the move to ubuntu. The step now asserts contents rather
than printing a listing, which is what it should have done — twenty
alphabetical CSS filenames are not evidence.

**Longhorn's three packages were never flipped.** All still carried
`private: true`, no `publishConfig`, no `license` and `files: ["src"]`. Poodle's
were flipped on 2026-08-09 and these were missed because Longhorn publishes at
step 4, after Poodle. `npm publish` would have refused outright; the quieter
failure was that the tarballs shipped no LICENSE.

Longhorn's own dry run is deliberately not attempted yet. Its
`bun install --frozen-lockfile` still fails on the machine-local Poodle path,
which is step 3 of this card, and every Longhorn CI job runs on macOS at ten
times the Linux rate. It runs after the repoint.

## Step 1 Complete — Poodle Published 2026-08-10

`@inflatable-cookie/poodle-core@0.1.0` and
`@inflatable-cookie/poodle-svelte@0.1.0` are on the public registry, the
repository is public, trusted publishers are configured, the credential is
dropped and `v0.1.0` is tagged.

**The registry holds exactly what the gates ran against.** The published
tarballs were downloaded from `dist.tarball` and compared to the CI artifact:

| Package | Bytes | SHA-256 |
| --- | --- | --- |
| `poodle-core` | 180,217 | `4934f660…02afd820` |
| `poodle-svelte` | 192,552 | `15208b85…2d8a5cb8` |

Byte-identical to run 31388087919's artifact. Publishing the artifact rather
than a locally repacked tarball is what makes that check meaningful.

**The tag path is validated.** Run 31391825299, dispatched against
`refs/tags/v0.1.0` with `dry-run=true`, is green on all thirteen steps
including *Versions agree with the tag* — which had never executed, because a
branch dispatch skips it. The dispatch-against-the-tag model works.

Provenance is absent from this release and will attach from the next one: it
requires a public source repository, and visibility flipped after the tarballs
were built.

Two consequences of going public are worth noting for later. Standard runners
are free for public repositories, macOS included, so the Actions-budget
pressure that forced dispatch-only is gone — restoring `pull_request` triggers
is now a choice rather than a cost. And the three uncancellable runs that held
the `main` concurrency group have cleared.

## Bootstrap Sequence — Poodle

Trusted publishing can only be configured on a package that already exists, so
the first publish is manual and every publish after it is OIDC. This is the
only time credentials touch the process, and they stay on the operator's
machine — `npm login` is browser-based and writes a token to `~/.npmrc`, not to
GitHub.

**1. Take the verified bytes from the green run.** Publish the artifact, not a
tarball repacked on a laptop: the artifact is what the gates ran against.

```sh
cd ~/Dev/projects/poodle
rm -rf /tmp/poodle-release   # gh run download refuses to overwrite
gh run download 31388087919 -n packed-tarballs -D /tmp/poodle-release
ls -l /tmp/poodle-release
shasum -a 256 /tmp/poodle-release/*.tgz
```

Verified 2026-08-10 against run 31388087919:

| Tarball | Bytes | Entries | SHA-256 |
| --- | --- | --- | --- |
| `inflatable-cookie-poodle-core-0.1.0.tgz` | 180,217 | 340 | `4934f660…02afd820` |
| `inflatable-cookie-poodle-svelte-0.1.0.tgz` | 192,552 | 175 | `15208b85…2d8a5cb8` |

Sizes match the pack step's own report, and both carry a LICENSE. The artifact
expires 2026-11-08; after that, re-run the dry run rather than packing locally.

**2. Authenticate.** Enable 2FA on the npm account first if it is not already
on — the whole point of trusted publishing is that no long-lived credential
exists, and for the one window where one does, it should be second-factor
protected.

```sh
npm login          # opens a browser; no token is stored in GitHub
npm whoami         # confirm before publishing anything
```

**3. Publish core, then svelte.** Order matters only for tidiness: svelte peer-
depends on core, and npm does not enforce peers at publish time.

```sh
npm publish /tmp/poodle-release/inflatable-cookie-poodle-core-0.1.0.tgz --access public
npm publish /tmp/poodle-release/inflatable-cookie-poodle-svelte-0.1.0.tgz --access public
```

`--access public` is explicit even though the manifests carry
`publishConfig.access`. A scoped package defaults to restricted, and a scoped
package published restricted on a free account fails rather than silently
going private — but being explicit costs nothing at the one irreversible step.

Add `--otp=<code>` if 2FA prompts non-interactively.

**4. Verify before touching anything else.**

```sh
npm view @inflatable-cookie/poodle-core version
npm view @inflatable-cookie/poodle-svelte version
```

Unpublish is available for 72 hours and only while nothing depends on the
package. After that a name can be deprecated but never reclaimed. If a tarball
is wrong, this is the window.

**5. Configure trusted publishers.** Web UI only; there is no CLI for it. For
each package: npmjs.com → the package → Settings → Trusted Publisher → GitHub
Actions, then

| Field | Value |
| --- | --- |
| Organisation | `inflatable-cookie` |
| Repository | `poodle` |
| Workflow filename | `release.yml` |
| Environment | leave blank |

**6. Drop the credential.**

```sh
npm logout
npm whoami   # must now fail
```

**7. Tag.**

```sh
git tag -a v0.1.0 -m "First publication"
git push origin v0.1.0
```

**8. Exercise the tag path without publishing.** The version-agrees-with-tag
check is skipped on a branch dispatch, so it has never run. Dispatch against
the tag with `dry-run=true` to fire it:

```sh
gh workflow run release.yml --ref v0.1.0 -f dry-run=true
```

**Do not dispatch v0.1.0 with `dry-run=false`.** Those versions are already on
the registry and the publish step would fail. The workflow's publish path first
runs for real at the next version.

**9. Clean up.**

```sh
git push origin --delete release-dryrun
```

Keep it if the three uncancellable runs still hold the `main` concurrency
group; it is the only lane that dispatches immediately.

### Provenance Waits For Public

Trusted publishing works on a private repository, but npm provenance
attestation does not — it requires the source repository to be public. The
first release therefore ships without provenance whichever way it is published.
Attestation starts at the first CI publish after visibility flips.

## Step 3 — The Clients Lane Passes — 2026-08-11

Every step of the CI `clients` job run locally, in order:

| Step | Result |
| --- | --- |
| `bun install --frozen-lockfile` | passes |
| type check every package | 3 of 3 |
| `scripts/test-packages.sh` | 194 pass, 0 fail |
| vitest | 104 passed |
| Svelte type gate | 1,416 files, 0 errors |
| generated bindings current | 13 of 13 domains |

Nothing in the lane was fixed. The install failed because the lockfile pinned
`file:../poodle/.artifacts/svelte-pack-install-V2Wu2n/…`, an ephemeral pack
directory Poodle mints fresh on every rebuild and which has never existed on a
clean checkout. Repointing to `^0.1.0` for Card 179 resolved it in passing.

That is worth naming: the lane was never broken. It was correctly reporting
that the repository could not be installed from a clean checkout, and it said
so for five days while the runway treated it as a lane problem.

## The Release Dry Run Is Green — 2026-08-11

`release.yml` had never executed. Its first run failed, and so did the next
five. Eight distinct defects, none of them visible to `effigy qa` here.

| Run | Reached | Defect |
| --- | --- | --- |
| 1 | 23s | `poodle-specs` path escaped the repository; `cargo metadata` failed for all 50 members |
| 2 | 7m | `boundary.test.ts` realpath'd sibling Poodle source |
| 3 | 10m | gpui prototypes took Poodle twice, by path and by tag; `verify-source-consumer.sh` probed the deleted `longhorn-layout` |
| 4 | 9m | seven proofs copied `[workspace.dependencies]`, nine pins stale, `--offline` could not resolve them |
| 5 | 20m | five proofs parsed vitest output that arrives coloured when `CI` is set |
| 6 | 21m | `rg` absent, in the greenfield proof and again in `check-release-floor.sh`, the second hiding a floor toolchain the workflow never installed |
| 7 | 31m | green |

Three tarballs packed and asserted: `longhorn` at 114,172 bytes and 143
entries with 31 generated bindings, `longhorn-poodle-svelte` at 52,494 and 79,
`longhorn-tauri` at 4,769 and 12. Each carries its LICENSE, README and
manifest. `packed-tarballs` uploaded. `Versions agree with the tag` was
skipped, correctly — it is gated on a tag ref and this ran against `main`.

Every defect came from a property of a developer machine that a runner does
not share: a sibling Poodle checkout, a cargo cache holding versions the
lockfile no longer names, `CI` unset so output is uncoloured, and ripgrep
installed. Two were helpers duplicated across proofs and rotted in the copies.
The reproductions are recorded in PAPERCUTS.md, which suggests folding them
into an `effigy ci:rehearse` task.

The step 3 note above said the clients lane had been correctly reporting for
five days that the repository could not be installed from a clean checkout.
The same was true of the whole repository, and only a workflow that had never
run was in a position to say so.

## The First Release Ships Against Poodle 0.2.0 — 2026-08-11

Decision: Longhorn's first published version depends on Poodle 0.2.0, not
0.1.0. Poodle is taking a large feature set to 0.2 rather than cutting a 0.1.1,
and Longhorn already needs unreleased Poodle -- `7827bd7d` forwards a DockRegion
`showTabs` prop that does not exist in 0.1.0.

**CI cannot run until Poodle 0.2.0 publishes, and that is accepted.** The
manifest declares `^0.1.0` while the code requires main, so `check:svelte`
fails on a clean install. This is a known state, not a regression to
investigate. Develop against `effigy deps link bun ../poodle`, which leaves the
manifest byte-for-byte unchanged.

The dry run recorded above stays valid for everything that is not the
TypeScript gate: the workflow, the packing assertions, the artifact upload and
the Rust half were all proved green on 2026-08-11 before this dependency
appeared.

### What Poodle main has already broken, and its state

| | |
| --- | --- |
| DockRegion `showTabs` | needs main; red against 0.1.0 |
| Tabs `variant="text"` | renamed `card` by 892c2e51 — fixed |
| package boundary test | asserted node_modules, now asserts the manifest — fixed |
| Svelte dev pin | was 5.38.6, the floor of the peer range; now 5.56.8, what every consumer runs — fixed |
| `SettingsShell` anchor focus | **open** |

The open one is not Longhorn's to fix. `resolves search anchors and focuses the
structural target` fails with focus on `body`, and diagnosis on 2026-08-11 put
it in Poodle's Dialog.

Dialog defers its focus restore one macrotask on the close edge (`b1a4a5e7`),
which is correct -- a pending keyup would otherwise re-activate the trigger and
reopen it -- but the timer handle is never stored and never cancelled.
`onDestroy` restores only `body` overflow. So a closed Dialog schedules a focus
change that fires unconditionally, undoing whatever the application did with
focus in that macrotask, and firing even if the Dialog has unmounted.

Proved by bisecting the file rather than by reading it:

| run | result |
| --- | --- |
| the test alone | passes |
| after `hosts over one controller` | passes |
| after `keeps dirty state behind the close guard` | fails |

The guard test opens and closes a Dialog. Probing showed the section *is*
focused correctly and then loses focus in a later turn, which is the stray
restore landing on a detached trigger.

That makes the failure a true positive against a real bug -- close a dialog,
move focus deliberately, and Poodle takes it back a tick later -- so the test
stays failing and SettingsShell is untouched. Raised with Poodle 2026-08-11.

124 of 125 vitest pass; Longhorn's own svelte-check errors against local Poodle
are zero, and the two that remain are in Poodle's own `AudioSwitch.svelte`.

### Consequences for the steps above

Step 4 (publish) and step 5 (tag) wait on Poodle 0.2.0. Step 6's consumers are
already developing against linked local Poodle and unpublished Longhorn, which
is why five of them now carry `file:` Longhorn references with an `overrides`
block; those revert to versions at the same time.

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
- every workflow in both repositories triggers only on `workflow_dispatch`
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
