# Scripts

Repo-owned automation that Effigy cannot express generically. Run everything
from the repo root:

```sh
effigy tasks
effigy doctor
effigy qa
```

## What runs where

Every script here is reachable from a selector. Nothing in this directory is
meant to be run by remembering it exists — if something is worth keeping, it is
worth wiring, and if it is not wired it should be deleted.

`proof:artifacts`, a member of `qa`, runs fourteen of them in order:

`scripts/fmt-rust.ts` runs under `fmt:rust` to format-check root workspace
members, all eight `prototypes/*/` workspaces (both via `scripts/rust-command.sh`),
and the staged example template Rust sources (`rustfmt --edition 2024 --check`).

`scripts/proof-artifacts.ts` keeps that order but runs every member even after
one fails (`scripts/proof-member-run.ts`), so a milestone run reports every broken
proof at once. It suppresses passing member stdout (printing failing output at
the end), prints each member's elapsed seconds and result to stderr, and exits
non-zero naming the failed members. Each member resolves its own Cargo storage:
checkout-level routes run through `scripts/rust-command.sh` and pool, and each
staged workspace leases its own private final outputs on external build scratch
through `scripts/private-cargo.sh`. It drops an inherited `CARGO_TARGET_DIR` so
a checkout-level route cannot be moved off the pool. A failed member is included
in the summary.

Artifact consumer stages install packed Longhorn tarballs with a private Bun
cache. Exact consumer pins stay exact; other third-party versions use the root
`bun.lock`. A staged version newer than the root lock fails the proof. The
newest-admitted pass of `verify-pack-typecheck.ts` deliberately resolves the
declared Svelte and Poodle peer ranges and reports the installed versions.
The root `@testing-library/jest-dom` dev pin locks matcher dependencies used
by the greenfield examples.

| script | proves |
| --- | --- |
| `verify-app-shell-proof.ts` | the composed shell mounts against both host adapters |
| `verify-bridge-topology-conformance.ts` | each topology example imports only its declared surface |
| `verify-bridge-topology-artifacts.ts` | the five topologies install and run as isolated graphs |
| `verify-settings-composition-proof.ts` | the settings composition example against the real registry |
| `verify-command-system-artifacts.ts` | the command system example, packed and installed |
| `verify-history-system-artifacts.ts` | linear history, packed and installed |
| `verify-history-tree-artifacts.ts` | fork history, packed and installed |
| `verify-operation-notification-artifacts.ts` | operations and notifications together |
| `verify-native-content-artifacts.ts` | native content, including the three Tauri examples |
| `verify-poodle-preview.ts` | the Poodle projection renders under both backends |
| `verify-greenfield-compositions.ts` | the greenfield compositions carry no donor vocabulary |
| `verify-guide-corpus.ts` | the guides match the generated API surface |
| `verify-documented-commands.ts` | every command the examples' READMEs name exists |
| `verify-pack-typecheck.ts` | the packed adapter typechecks against a bare stage whose only Poodle source is the registry, at both the newest and the lowest release each framework peer range admits (Svelte and Poodle) |

Artifact verifiers share command execution, file dependencies, digests and
inventory assertions in [`proof-support.ts`](proof-support.ts).

`proof:guide-corpus` runs `verify-guide-corpus.ts` on its own and remains
one of the fourteen `proof:artifacts` members. The maintained
`longhorn-getting-started-docs` group pairs it with the guide link checks.

The rest are wired individually: `check:api-reference` runs
`generate-api-reference.ts`, and `verify-held-surface.ts`,
`verify-host-protocol.ts` and `verify-private-candidate-docs.ts` are
each their own selector. `check:agent-control-skill` runs
`verify-agent-control-skill.ts` (skill-vs-vocabulary drift, finder fixtures,
install fixtures). `check:config-age:security` runs
`verify-config-age-security.ts`: it compiles `longhorn-config-age`
all-targets and stages disposable Git-dependency consumers at the exact tree
to prove the repaired age line (RUSTSEC-2026-0387) is what public consumers
resolve, that a stale consumer lock resolved on the old line is refused
under `--locked`, and that one re-lock lands clean.
`check:native-content:local` and
`proof:native-content:local` run `verify-native-content-local.ts` (`--check-only`
and plain): they stage the child-view crate with a product-free native
consumer into a disposable workspace — the check compiles it at the MSRV pin,
the proof runs the real macOS application that attaches one bundled local
document through the public API (contract 017; macOS with a GUI session). `agent-control:install-skill` copies
`skills/agent-control/` into a consumer repo
(`effigy agent-control:install-skill -- <git-repo>` or
`effigy agent-control:install-skill -- --repo <git-repo>`); the bun script
remains the implementation. `check:agent-control-shim` builds the webview IIFE
from the TypeScript injection entry and checks the committed Tauri asset byte
for byte; `generate:agent-control-shim` writes that bundle after source edits.
`bootstrap:deps` runs the locked Bun install fresh worktrees need before
TypeScript checks; `check:bun-deps` guards
`check:ts`/`check:svelte` when run alone. `consumer-absence.ts`,
`poodle-release.ts`, `proof-install.ts`, and `longhorn-version.ts` are shared
modules, not entry points. `rust-toolchain-channel.sh` is invoked by `ci.yml` and
`release.yml`, not by a selector: it prints the pinned stable channel from
`rust-toolchain.toml` so both workflows install the declared version instead
of floating `stable`. `git-snapshot.sh` is shared by `verify-source-consumer.sh`
and `verify-config-age-security.ts` to snapshot dirty working trees into a
deterministic Git commit. `release:bump` and `check:release-gates` are the
release-tooling selectors; see Release below.

`check:agent-tool-dispatch` runs the focused contract-023 crate tests, its
release/default contract-022 absence proof, and the generated API check.
The bounded `agent-tool-dispatch` group selects this aggregate. Package-scoped
TypeScript groups use `check:ts:<package>` and `test:ts:<package>` selectors;
`scripts/test-packages.sh` keeps its aggregate behavior with no argument and
accepts one package directory for its Bun-native package suites. The Svelte
package selector uses its existing Vitest config.
`check:bun-links` (wired into `ci:rehearse`) reports the machine's bun
global-link state and fails when a dependency resolves through a link that
leaves the repository.
`proof:agent-tool-dispatch-source-consumer` additionally requires
`AGENT_LONGHORN_REPO` and `AGENT_PROJECTS_ROOT`; it accepts only a clean
Longhorn descendant of the promoted base and exact clean Swallowtail
`d7e93e5552c5b272e55ddef8a531b5dd32e81bf0`, then compiles and runs the
provider-free fixture outside both repositories.

## No consumer knowledge

Longhorn keeps no code with direct knowledge of a consuming application. No
script resolves a sibling repository or reads a consumer's manifest, and no
standing proof outside the disposable g02.036 source-consumer acceptance
runner takes a `*_REPO` environment override: the Poodle-pack escape
(`POODLE_REPO`) stopped being consumed when greenfield moved to the registry
release, while `AGENT_LONGHORN_REPO` and `AGENT_PROJECTS_ROOT` stage that
fixture in `mktemp` rather than letting a gate resolve a live sibling. Twenty-
three verifiers that did were deleted on 2026-08-10, along with the frozen
receipts they asserted against; both remain in git history.

Consumer *names* may still appear as vocabulary — a test graph shaped like a
real application's, an example named for the topology it exercises. What may
not appear is a path, a pinned consumer commit, or an assertion about another
repository's working tree. A proof that reads a second repository can only be
green when both are simultaneously in one state, which is not a property this
repository can hold.

The g02.036 L1 acceptance fixture is deliberately different: it is created in
`mktemp`, consumes exact clean Longhorn and Swallowtail source SHAs through
machine-local symbolic keys, writes no dependency edge into either repository,
and emits only a sanitized one-run receipt. It is acceptance evidence for the
transport-neutral source seam, not a standing consumer claim or adoption pin.

`check:consumer-isolation`, a member of `qa`, enforces this. Outside the single
g02.036 source-consumer acceptance runner, it names no consumer — it detects
the mechanism, so a consumer arriving or leaving never requires editing it.
Three things fail it:

- a `*_REPO` environment override
- an absolute path into another checkout
- `resolve(repoRoot, "../name")` onto a sibling

Poodle and poodle-specs are admitted by name in `DEPENDENCIES`, because
Longhorn projects into them rather than the other way round. A fourth entry
should be argued for.

`check:repo-containment`, also in `qa`, is stricter about location: Cargo
`path`, package `file:` / `link:`, and `join(repoRoot, "../…")` must resolve
inside this tree. Its allowed-escape list names only the disposable g02.036
source-consumer runner described above; no standing consumer proof may take a
`*_REPO` override.

`verify-greenfield-compositions.ts` separately enforces the absence of donor
vocabulary inside the greenfield example root.

## CI rehearsal

`effigy ci:rehearse` reproduces the six developer-machine properties that hid
defects from local `qa` during the first release runs:

1. `effigy bootstrap:deps` — the locked Bun install CI runs before its proofs;
   a fresh worktree may have no `node_modules`, so the rehearsal installs it
2. `check:repo-containment` + `check:consumer-isolation` — no sibling path
3. `check:bun-links` — no active bun global link; registered ones are reported
4. `check:runner-tools` checks scripts for tools absent from clean runners,
   validates release-gate tool installs, and checks the Rust CI jobs' Bun pins.
5. `rust-command.sh fetch --locked` under a fresh `CARGO_HOME`, then offline
   metadata through the same switch point
6. `CI=1 effigy proof:artifacts` under that `CARGO_HOME` — coloured vitest output
   and a cold cache

Not a substitute for dispatching `release.yml`. Run it before a release when
`qa` is green and you want the clean-runner reading, not the developer one.

The hosted `clients` job in `ci.yml` runs only part of `qa`'s TypeScript and
Svelte family: `check:ts`, `test:ts`, `test:vitest`, `check:svelte` and
`check:bindings`. `lint:ts`, `fmt:ts` and `check:knip` run through `qa`, which
`release.yml` runs. `check:ci-static-parity` (`scripts/check-ci-static-parity.ts`) records that split
and fails when the job and the `qa` list drift apart. It reads the job's steps
through a YAML parser and counts only commands that certainly run: a step with
an `if`, an `echo` line, a step name, a comment, or code after a top-level
`exit` or inside an `if`/loop body does not count. `release.yml` must run
`effigy qa` by the same rule, in any of its jobs. Closing the gap in `ci.yml`
needs Tom's approval, so it is recorded as a follow-up rather than changed here.

`check:markers` (`scripts/check-markers.ts`) ports the retired
`scan attention-markers` rules, with the marker lists the retired
`[scan.attention_markers]` table set: scope by extension and path, generated
artifacts skipped, a marker that starts a comment body, one finding per marker
per line, and the deprecation, temporary and deferred categories. `BUG` and
`remove before release` fail the gate. High and warning findings are counts.

`check:generated-sources` (`scripts/check-generated-sources.ts`) ports the
retired `scan generated-in-src` rules over the scanner's source roots, in the
scanner's order: a generated header, a `.map` suffix (source map), a `.min.` name
(minified asset), a generated file name, and a `generated` or `gen` path
component. Severity classes come from byte size. Generated output under
`packages/*/src/**/generated/`, the bindings output in `crates/*/bindings/`, and
the bindings generator sources in `crates/longhorn-bindings/src/` are the
allowed surface. Outside the source roots, the generator header may appear only
on that allowed surface. Every file in a generated directory must be named by a
generator, and every text file there must carry the header. JSON output is
checked by its generator path alone.

`scripts/scanner-heuristics.ts` holds the two helpers both ports share: the
string-literal mask and the generated header test. Each rule has a synthetic
fixture test. Each port was also checked against the stable scanners on one
fixture that covers every rule: 15 of 15 generated-in-src findings and 19 of 19
attention findings matched.

## Escape hatches

- `KEEP_HISTORY_TREE_PROOF=1` retains the fork-history proof's disposable
  artifact workspace.
- `KEEP_GREENFIELD_COMPOSITION_PROOF=1` does the same for the greenfield
  composition proof.
- `KEEP_PACK_TYPECHECK=1` retains the pack-typecheck stages (one per entry
  in the peer matrix: the packed tarballs, the registry install, and the
  unpacked adapter tree).

None attempts registry publication.

## Rust routing

Every ordinary local Longhorn route runs Cargo through Oxide
(Tom, 2026-10-25). `scripts/rust-command.sh` is the one switch point, and it has two forms:

- `<cargo command> [args...]` — one Cargo command.
- `exec [--manifest-path <Cargo.toml>] [--profile <p>] -- <tool> [args...]` —
  one tool that calls Cargo itself, in one lease over one workspace and one
  toolchain.

Locally it runs `oxide` or `oxide exec`; with `GITHUB_ACTIONS=true` it runs plain
`cargo` or the tool itself, so hosted CI keeps the Cargo behavior it always had.
There is no per-selector `:cargo` task and no silent fallback: install Oxide, or
set `EFFIGY_OXIDE_ESCAPE=cargo` for one local run (any other value is refused),
or hand-run Cargo yourself for a one-off investigation. `fmt:rust` and the
`dependencies` release gate are switched too: they run non-building Cargo subcommands
(`fmt`, `metadata`, `deny`) that Oxide passes straight through.

`scripts/rust-command.ts` builds that argv for scripts and adds no rule of its
own: `rustCommand` for a Cargo command, `rustExec` for a leased tool,
`privateCargoCommand` for a leased tool that keeps private final outputs.

### Storage and lease boundaries

Oxide owns local Cargo output placement. A route that needs a private directory
keeps final outputs on external build scratch and never on the internal drive,
the account home or a drive root:

- `OXIDE_TASK_SCRATCH_ROOT`, when set, wins. It must already exist. Missing
  scratch fails the route; nothing creates it, so an unmounted build volume
  cannot turn into an ordinary directory on the internal drive.
- When it is unset, the default is the scratch root Oxide reports for its
  registered removable build pool (`oxide pool describe --json`,
  `removable.suggested_scratch_root`, only when `removable.available` is true).
  Nothing is hard-coded to a volume name. Any gap (no Oxide, no available pool,
  no suggested root, or a missing directory) fails closed and names
  `OXIDE_TASK_SCRATCH_ROOT`. The repository creates only a `longhorn` child below
  the default root. The child must be an owned real directory whose resolved
  path stays below that root.
- The resolved root, configured or reported, is refused when it is a drive root,
  a boot-volume top-level directory, a volume mount root, the account home, or on
  the boot volume. The internal drive is judged by volume
  (the device of `/` or of the account home), never by `TMPDIR`: Nucleus points
  `TMPDIR` at external scratch, and a `TMPDIR` on the boot volume is still refused.
  `scripts/rust-command.ts` owns this rule (`taskScratch`, and the `scratch`
  command that `scripts/external-scratch.sh` runs for shell routes).
- Hosted CI keeps its runner-owned temporary storage.

`oxide exec` replaces a `CARGO_TARGET_DIR` inherited on the outer call with
`<workspace>/target`, so the private directory survives only when it is set
inside the lease: `scripts/private-cargo.sh` is the tool behind the lease, it
exports `CARGO_TARGET_DIR` for the plain Cargo child it runs, and it makes no
routing decision of its own.
Setting either variable on the outer `exec` call, or in a proof's member
environment, does nothing. Only `CARGO_TARGET_DIR` moves final outputs: an
inherited `CARGO_BUILD_BUILD_DIR` still sends intermediates to the held slot, so
an inner target is not a second build store.

One lease covers one Cargo workspace and one toolchain. A proof that stages
several consumers, or builds at the MSRV, runs one `exec` boundary per staged
workspace: that workspace's `--manifest-path`, and `RUSTUP_TOOLCHAIN` pinned
before the lease. Inside a lease the tool calls plain `cargo`, never oxide: a
nested lease takes its own slot and can deadlock the host permits.

Staged workspaces need `prepareStagedWorkspace`: it creates an empty real
`target/` directory in the owned disposable workspace before the lease. An
existing real target opts that workspace out of Oxide's automatic first
conversion — which otherwise refuses a workspace with no Git work tree to
confirm — reports `outputs not converted`, and creates no Oxide output root.
Never apply it to a real checkout, never overwrite an existing target or an
owned-output link, and do not substitute `git init` or target-ignore metadata
for it. The conversion refusal is an Oxide defect (upstream `755757db`); this is
the supported interim rule on the installed Oxide (`4df3dfd`). Real Rust stages
live on external build scratch and are removed after use: proofs allocate them
with `stagingDirectory` or `taskScratch` in `scripts/rust-command.ts`, never
under `os.tmpdir()`, and `scripts/staged-storage.test.ts` fails any proof that
does.

### Cargo entry points

| Route | Form |
| --- | --- |
| Every `lint:*`/`test:*`/`docs:*` Rust selector, `check:prototypes`, `check:bindings`, `release:floor`, `ci:rehearse` | `rust-command.sh <cargo command>` |
| `effigy fmt:rust` | `rust-command.sh fmt --check -p …` (workspace), `rust-command.sh fmt --check` (prototypes), `rust-command.sh metadata --no-deps`, plus `rustfmt --edition 2024 --check` on staged example sources |
| `[release.gates] dependencies` / `effigy release:gates` | `rust-command.sh deny check` |
| `check:machete` | `rustCommand` for workspace metadata and `machete` |
| `scripts/sync-prototype-locks.ts`, `scripts/prototype-dependency-coverage.ts`, `scripts/generate-api-reference.ts` | `rustCommand(["metadata", …])` |
| Checkout-level `package --list` inventories, bindings `run` checks, `tree` reads and `-p` tests in the proofs and the two absence scans | `rustCommand([...])`, at `RUSTUP_TOOLCHAIN=<MSRV>` where the call was `cargo +<toolchain>` |
| Staged and disposable consumer builds: the `scripts/proof-artifacts.ts` members, `scripts/verify-desktop-support-conformance.ts`, `scripts/verify-agent-tool-dispatch-source-consumer.ts`, `scripts/verify-source-consumer.sh` | `rustExec(privateCargoCommand(...))`, one boundary per staged workspace, at the MSRV pin where the staged crates need it |
| `check:agent-control-release-absence`, `check:agent-tool-dispatch-release-absence` | `rustExec(privateCargoCommand(...), { profile: "release" })`; the scan reads the rlib paths Cargo reports in `--message-format=json` output (`libraryArchives`), not a directory layout |
| `cargo tauri build` in the example and prototype READMEs | documented as `oxide exec -- cargo tauri build`; source guidance only, no live Tauri run is claimed |

Each feature state of the release-absence scans is its own private final output
directory, so no scan pins `CARGO_BUILD_BUILD_DIR`: intermediates pool like every
other staged consumer's, and Cargo keys them by feature.

Genuine Cargo exceptions:

| Route | Category |
| --- | --- |
| `.github/workflows/*`, including its `cargo metadata` step | CI, and workflow edits need approval |
| Effigy's own `cargo update --workspace` lockfile sync from `config/release.toml` | Not a Longhorn entry point; Effigy runs it |
| `scripts/check-runner-tools.ts` | Read `cargo deny`/`cargo install` as text to check workflow installs; not entry points |
| `scripts/workspace-dependencies.ts` | Reads `Cargo.toml` text; runs no Cargo |
| A consumer's own project commands in `docs/guides/` | Product Cargo behavior in the consumer's checkout, not this repository's route |
| `.git/queue-hooks` warm-merge manifest | Nucleus-owned, outside the repository |

## Language

Prefer Effigy for generic operations. If Longhorn needs repo-owned automation,
use TypeScript with Bun. Bash is thin glue only; Python needs a concrete
technical reason.

## Release

The release procedure is owned by
[`docs/knowledge/contracts/release.md`](../docs/knowledge/contracts/release.md).
