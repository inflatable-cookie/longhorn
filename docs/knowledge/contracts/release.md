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
`clippy::chunks_exact_to_as_chunks`, green on 1.97.1 locally and red on the
runner, and stopped a dry run before publish.

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
   A nonzero exit on an empty `[Unreleased]` is a known Effigy gap (a Queue
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
declaration order: private-candidate, advisories, rustdoc, prototypes,
workspace (`effigy qa`), floor (MSRV Clippy and tests), source (git-dependency
consumer). `effigy release:gates` is the same list minus `workspace`;
`check:release-gates` fails if the two drift. `check:runner-tools` derives
extra tools from those commands (`cargo deny` needs cargo-deny) and fails if
`.github/workflows/release.yml` does not install them in an unconditional
step of the same job, before `effigy release:gates`; a comment is not an
install. It also checks that `ci.yml`'s `rust` and `rust-floor` jobs install
the same SHA-pinned Bun action and version as `clients` before their first
Effigy step, because the Rust test suites run a Bun script. The
`private-candidate` gate reads version, Rust crate count, and
consumer-graph count from the frozen receipt; CHANGELOG and the
compatibility guide must still carry those facts, not a fixed phrase.
`effigy test:release-tooling` covers the bump, the alignment check, that
mapping, and this fact check.

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

The yanked `yoke-derive` 0.8.3 is replaced by 0.8.4 in the root and affected
prototype locks. The targeted Cargo update also moves `cssparser-macros`
0.7.1's `syn` edge from 3.0.6 to 2.0.119 where both are already locked.
Its `>=2, <4` requirement permits both. The planner accepted this graph
re-resolution on 2026-10-18: no package is added or removed, and only
`yoke-derive` changes version.

`tokio` 1.53.1 and 1.53.2 are replaced by 1.53.3 for RUSTSEC-2026-0386
(`AsyncFd` constructors violated I/O safety) in the root lock and the six
affected prototype locks: `agent-control`, `gpui-composition`,
`gpui-windowing`, `native-content-backing-surface`,
`native-content-child-webview` and `native-content-isolated-window`. The
manifests' `1.45.1` requirements and 1.53.3's `rust-version` 1.71 already
admit it, so no manifest changes and no `deny.toml` allowance. Tom approved
this bounded update for 0.4.3 on 2026-10-23 (decision `e4881db7`): only
`tokio` changes version and checksum; no package is added or removed and no
dependency edge moves.

Those locks are Longhorn's own. The Rust crates are `publish = false` and
consumers take them by git tag, so the tag carries the requirement, not the
resolution: a consumer's own `Cargo.lock` keeps whatever Tokio it already
resolved until that consumer updates it, and it needs a patched Tokio
resolution (`>= 1.53.3`, or `>= 1.51.6, < 1.52.0`) there too. Longhorn neither
writes consumer repositories nor raises the `1.45.1` manifest requirement to
force that upgrade.

A bare `cargo update` on this workspace also rewrites 11 pre-existing
`windows-sys`/`syn` dependency-qualifier edges in the root lock (and two in
four of the affected prototype locks): on rewrite Cargo re-derives each edge
from the registry and picks a different version that is already locked. No
package version, checksum or package set changes, but the rewrite is unrelated
to the advisory, so a lock-only security patch keeps the edges the version
bump wrote instead of folding an unrelated re-resolution into its diff.

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
