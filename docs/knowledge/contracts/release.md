# Release

Longhorn releases one coordinated version across the Rust workspace and the
three npm packages. The npm packages publish to npm under `@inflatable-cookie`
through trusted publishing. The Rust crates are `publish = false`; consumers
take them by git tag, so the Rust half of a release is the tag itself.
Tom approves each release. What a candidate must prove is owned by
[contract 012](012-distribution-and-compatibility.md#release-evidence).

## Version source

The version lives in `Cargo.toml` `workspace.package.version`. Proofs and
boundary tests read that field. Do not hand-edit version literals in
`scripts/` or the adapter peers. `config/release.toml` is the only
`[release]` and `[release.gates]` definition; `effigy.toml` includes it.

## Toolchain pin

The stable toolchain Longhorn builds, tests and releases with is pinned once,
in `rust-toolchain.toml` at the repository root. rustup reads it locally, and
`ci.yml` and `release.yml` parse the channel through
`scripts/rust-toolchain-channel.sh` and install exactly that version. Floating
`stable` is banned from workflows: Rust 1.98 added
`clippy::chunks_exact_to_as_chunks`, which stopped a dry run before publish.

The MSRV floor is separate. `release-baselines/rust-toolchains.env` holds
`LONGHORN_GENERAL_MSRV`, which the floor gate exercises through `rustup run`.
Never put the stable channel in that file, and never put the floor in
`rust-toolchain.toml`.

To bump the pinned toolchain:

1. Set `channel` in `rust-toolchain.toml` to the new stable version.
2. Install it, then run `effigy qa` and `effigy release:floor`. Fix any new
   lint in the same PR: a new Clippy lint on pre-existing code is the expected
   cost of a bump.
3. Dispatch `ci.yml` and confirm the run logs `rustc --version` at the new
   channel.

## Changelog fragments

Unreleased changes are one fragment per entry, in
`changelog.d/<task-or-PR>-<slug>.<type>.md`. `type` is `added`, `changed`,
`fixed`, `removed` or `security`: a public-API break is `changed`, and a
credential or data leak fix is `security`. The stem carries a task or PR
reference and a short slug so branches never collide. A fragment holds one
paragraph without list markers; split the change into another fragment instead
of adding a blank line. `effigy skill run northstar-docs/changelog -- check`
is the check; the release fold reads the same parser. Folding rewrites them
into `CHANGELOG.md` under the typed headings and deletes them.

`scripts/check-changelog.ts` judges `CHANGELOG.md` alone, with Effigy's release
parser, in a staged copy. It reports a refused release Run as a note rather
than a changelog failure: release Run state lives in Effigy machine state
outside this repository (longhorn#174).

## Steps

1. **Bump.** `effigy release:bump -- <version>` updates the workspace version
   and internal pins, the three npm versions and adapter peers, the
   agent-control skill stamp, promotes `[Unreleased]` in `CHANGELOG.md` to a
   dated heading, runs `sync:prototype-locks`, and regenerates the API
   reference. It reports every file it changed, including lock rewrites. It
   is idempotent at the same version and refuses a non-increasing one. A
   package manifest whose `"version"` line is not the expected shape is a
   stop. A lock change other than a Longhorn path-package version is a stop.
   A failure partway through leaves the tracked tree unchanged. It never tags.
2. **Land the bump** as its own reviewable PR. Do not commit a scratch bump
   used only as evidence.
3. **CI on the exact commit.** CI is manual dispatch:
   `gh workflow run ci.yml --ref <branch>`. Wait for green.
4. **Rehearse.** `effigy ci:rehearse` on that commit. It checks the
   clean-runner properties that local `qa` can miss.
5. **Gates.** `effigy release status --check-gates` and read the gate lines.
   A nonzero exit on an empty `[Unreleased]` is a known Effigy gap (a Nucleus
   papercut); the gate results are the evidence.
6. **Dry run.** `gh workflow run release.yml --ref main -f dry-run=true`.
   It runs full QA, the release gates and pack-and-verify.
7. **Tag and publish.** Dispatch the workflow against the tag, not `main`:

   ```sh
   git tag -a v<version> -m "Longhorn <version>" && git push origin v<version>
   gh workflow run release.yml --ref v<version> -f dry-run=false
   ```

   There is no `push: tags` trigger: publication is an act someone performs,
   not a side effect of pushing a tag.

## Gates

`[release.gates]` in `config/release.toml` runs cheapest-first, in
declaration order: private-candidate, dependencies, rustdoc, prototypes,
workspace (`effigy qa`), floor (MSRV Clippy and tests), source (git-dependency
consumer). `effigy release:gates` is the same list minus `workspace`;
`check:release-gates` fails if the two drift. `check:runner-tools` derives
extra tools from those commands, through the effigy task each one runs and the
scripts it invokes (`cargo deny` needs cargo-deny), and fails if
`.github/workflows/release.yml` does not install them in an unconditional
step of the same job, before `effigy release:gates`; a comment is not an
install. The dependency gate checks advisories, bans, licenses and sources
using `deny.toml`. Duplicate versions are surfaced as warnings.
`check:runner-tools` also checks that `ci.yml`'s `rust` and `rust-floor` jobs install
the same SHA-pinned Bun action and version as `clients` before their first
Effigy step, because the Rust test suites run a Bun script. The
`private-candidate` gate reads version, Rust crate count, and
consumer-graph count from the frozen receipt; CHANGELOG and the
compatibility guide must still carry those facts, not a fixed phrase.
`effigy test:release-tooling` runs every `scripts/**/*.test.ts` on disk, so the
bump, the alignment check, that mapping and this fact check are covered without a
hand-kept list.

Effigy `release prepare` cannot see the workspace-excluded
`prototypes/*/Cargo.lock` files. `sync:prototype-locks` rewrites Longhorn
path-package versions or copies a missing Longhorn path package and its
dependency edges from the root lock. It refuses any third-party lock
movement, then proves each lock with
`cargo metadata --locked --offline --no-deps`. Run it before a task-time
prototype check if a path package or version changed. Keep it
out of `[release.gates]`; do not weaken `check:prototypes --locked`.

When bumping `LONGHORN_GENERAL_MSRV`, run `effigy release:floor` in the same
change. The floor unlocks MSRV-gated Clippy lints. The pinned stable channel in
`rust-toolchain.toml` is a separate declaration with its own bump procedure
("Toolchain pin" above).

## Lock maintenance

Use targeted dependency updates. Review package versions, checksums and
qualified edges in the root and affected prototype locks. A bare Cargo
update can rederive unrelated `windows-sys`, `getrandom` or `syn` edges even
when their versions are already locked. Keep unrelated re-resolution out of a
bounded security fix.

A lock-only advisory fix updates Longhorn's resolution. Applications taking
the crates by Git tag can keep an older version in their own lockfile and
need to resolve the patched version separately. When no safe version fits
the manifest requirement, coordinate the requirement change and prove both
fresh resolution and any affected compatibility fixtures.

The [changelog](../../../CHANGELOG.md) records per-release lock changes and
publication evidence. Getting Started follows the latest version with a
`Publication evidence` entry. A version heading added during preparation
records a candidate.

## Publishing credentials

No `NPM_TOKEN`. npmjs.com accepts publishes for each package only from this
repository and `release.yml`, authenticated by a per-run OIDC token, with
provenance attached. A new package needs its trusted publisher configured at
npmjs.com before its first release; until then the publish step fails closed.
CI and release workflows pin Effigy `0.14.1` through
`inflatable-cookie/setup-effigy`; bump both files together. This version reads
the top-level `[qa]` groups in `effigy.toml`. Never use `bunx effigy` there:
an unrelated npm package has that name.

Both workflows set `EFFIGY_SCHEDULER_OVERRIDE` to
`GitHub-hosted runner: one job per VM, no shared host scheduler`. Effigy 0.14
routes heavy selectors to a host scheduler; GitHub-hosted VMs have none.
This is the audited override route in Effigy guide 080, approved by Tom on
2026-10-18 (decision `de2de5cb`). Effigy papercut `25ead2bf` asks for a CI mode.
Local validation keeps host-wide admission.

## Verify

- The release run is green: full QA, gates, tag-version check,
  pack-and-verify, publish.
- `npm view @inflatable-cookie/<package>@<version> dist.integrity` equals the
  packed artifact from the release run, for all three packages.
- Both adapters peer on `@inflatable-cookie/longhorn` at the exact version.
- The annotated tag dereferences to the released commit.
- Send consumers the exact pins: npm versions and the git tag.

## Roll back

- npm versions are immutable. Fix forward with a patch release. Use
  `npm deprecate` on a bad version if consumers must be steered away.
- Never move or delete a pushed tag; consumers pin it. Cut a new one.
- A failed publish run that published nothing can be re-dispatched against
  the same tag.
