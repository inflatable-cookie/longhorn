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

`age` moves from 0.12.1 to 0.11.5 for RUSTSEC-2026-0387 (crypto-bigint
constant-time execution issues; patched only in >= 0.7.6, outside the 0.5
line age 0.12 reaches). The repair is a manifest requirement change, not a
lock-only bump: age 0.12.x carries a mandatory hpke → p256 →
elliptic-curve → crypto-bigint path in every `default-features = false`
build and no published 0.12.x release drops it, while age 0.11.x has no
mandatory p256/hpke dependency at all. `longhorn-config-age` uses only the
X25519 and scrypt recipients both lines implement, so no source change was
needed; the compatibility contract is the age v1 file format (contract
004), and retained ciphertext captured through the public API on 0.12.1 —
the line 0.4.3 shipped — is frozen in `fixtures/config-age-compat` and
restores under 0.11.5 (`test:config-age:security`). The targeted
`cargo update -p age --precise 0.11.5` moved: age 0.12.1 → 0.11.5,
age-core 0.12.0 → 0.11.0, bech32 0.11.1 → 0.9.1, fluent 0.17.0 → 0.16.1,
fluent-bundle 0.16.0 → 0.15.3, fluent-syntax 0.12.0 → 0.11.1,
i18n-embed 0.16.0 → 0.15.4, i18n-embed-fl 0.10.1 → 0.9.4, nom 8.0.0 →
7.1.3; removed twenty packages (aes-gcm, base16ct, crypto-bigint, ctr, der,
elliptic-curve, ff, ghash, group, hpke, keccak, kem, ml-kem, p256, polyval,
primeorder, proc-macro-error-attr3, proc-macro-error3, sec1, sha3) and
added three leaves of the moved lines (minimal-lexical,
proc-macro-error-attr2, proc-macro-error2). No unrelated edge moved — in
particular none of the pre-existing `windows-sys`/`syn` qualifier edges.
No prototype lock carries age, so no prototype lock moves.

The advisories gate then failed on a new error the downgrade exposed:
i18n-embed-fl 0.9.4 — the newest release satisfying age 0.11.5's `0.9`
requirement — depends on `proc-macro-error2`, unmaintained with no patched
release (RUSTSEC-2026-0173). The repair does not add an allowance for it;
it pins i18n-embed-fl to 0.9.1, the newest `^0.9` release that still uses
the original `proc-macro-error` 1.0.4 already locked through the
glib/gtk build chain and already covered by the existing RUSTSEC-2024-0370
allowance. That second targeted update (`cargo update -p i18n-embed-fl
--precise 0.9.1`) moves only i18n-embed-fl 0.9.4 → 0.9.1, removes
proc-macro-error2 and proc-macro-error-attr2, and adds dashmap 6.2.1 and
hashbrown 0.14.5 (i18n-embed-fl 0.9.1's own leaves; hashbrown joins three
pre-existing copies under `multiple-versions = "warn"`). `basic-toml`
stays reachable through `i18n-config`, so the RUSTSEC-2026-0374 allowance
keeps matching. The gate runs green with no new advisory ignore, no
removed gate, and no manifest change beyond the age line. The
`rustc-hash` 1.1.0 and `self_cell` 0.10.3 duplicate copies the moved fluent
line adds sit under the existing `multiple-versions = "warn"` posture; no
new `deny.toml` allowance. The workspace `rust-version` 1.95 floor is
preserved (age 0.11.5 declares 1.65). Consumers taking the crates by git
get the repaired requirement from the manifest, so their own fresh
resolution lands on 0.11.x with no vulnerable chain; a consumer lock
resolved against the old line is refused under `--locked` and re-locks
clean (`check:config-age:security` proves both). A consumer that pins its
own separate age 0.12.x direct dependency keeps that copy as a
semver-incompatible duplicate for its own use — no manifest requirement
can prevent that, and Longhorn's guarantee covers the resolution it
participates in.

Both repairs touch only Longhorn's own locks. The Rust crates are
`publish = false` and consumers take them by git tag, so the tag carries
the requirement, not the
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

## 0.5.0 candidate

Tom authorized preparing the 0.5.0 candidate on 2026-11-02 (decision
`cc2087bb`): the reviewed bump-preparation slice only. That decision never
authorizes workflow dispatch, tagging, or publication; those stay gated by
the steps above. The release carries one coordinated pre-1.0 source break:
`ChildViewSpec::new` takes a `ChildViewSource` instead of a bare `Url`, and
`ChildViewSpec::source` returns the typed carrier (contract 017). External
callers migrate through `ChildViewSource::external` — the historical remote
HTTP/HTTPS behavior — or `ChildViewSource::bundled`, a validated relative
path inside the application bundle served by the application asset protocol.

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
