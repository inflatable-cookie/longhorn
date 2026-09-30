# Validation Input Map

Status: active first pass  
Owner: Tom  
Updated: 2026-09-30
Gates: `effigy.toml`, `config/release.toml`
Procedure: [release](contracts/release.md)

## Purpose

One map from a changed input to the existing Longhorn selectors that should
run for it. Effigy owns the future selection contract; this file is Longhorn's
statement of what its gates actually cover. It enables no automatic omission
or execution by itself.

Read it with two rules:

- A selector is a coarse key, not a proof of completeness. Most Longhorn
  selectors are workspace aggregates, so a "narrow" answer is often still the
  whole Rust or TypeScript board.
- Effigy's graph `affected` results are candidates. The graph skips
  ignored/generated paths, so they cannot prove a generated or opaque input is
  covered. `graph affected` never widens a declaration here.

## Gate surfaces

`qa` is the board: twenty-four steps, run whole by `effigy qa`. The steps that
carry a selector are the selection keys; `bootstrap:deps` and `fmt:rust` are
steps too. `health` is `effigy doctor`'s cheap orientation subset and is not
full validation.

`[release.gates]` in `config/release.toml` is `effigy qa` plus release-only
work (`rustdoc`, `prototypes`, `floor`, `source`, private-candidate,
advisories). `check:release-gates` fails if `effigy release:gates` drifts from
that list.

## Selector detail

Field meaning: **role** (test / compile / format / proof / scan / docs);
**propagation** (what a dependency change drags in); **companions** (other
selectors the same change needs); **admission** (`heavy` = host-wide Effigy
lease; unmarked = no lease, ordinary priority).

### Rust

**`fmt:rust`** — role: format. Runs `cargo fmt --check` over `cargo metadata
--no-deps` members only, so it never walks a path dependency into a sibling
repo. Owns every workspace member's Rust source: `crates/*/{src,tests,
examples,benches}/**`, `crates/*/build.rs`, `examples/*/src-tauri/**`,
`examples/*/rust/**`, `examples/nucleus-no-surface-proof/**`,
`examples/packaged-update-proof/**`, `examples/update-licence-proof/rust/**`.
Propagation: per-file; manifest add/remove changes the member set.
Companions: none. Admission: unmarked (seconds). In `health` and `qa`. Limits:
prototypes are separate workspaces; does not compile.

**`lint:rust`** — role: compile/lint, default features. `cargo clippy
--workspace --all-targets --locked -- -D warnings`. Owns the Rust workspace
source set plus `Cargo.toml`/`Cargo.lock` (`--locked`). Propagation:
workspace-wide; any crate change compiles its dependents. Companions:
`lint:rust:features` (disjoint feature-gated code), `test:rust`. Admission:
unmarked (minutes). In `qa`. Limits: default features leave
bindings/supervision/operation and proof-example surface-mode code
uncompiled.

**`lint:rust:features`** — role: compile/lint, `--all-features`. Same inputs
and propagation as `lint:rust`. Companions: `lint:rust`. Admission: unmarked
(minutes). Limits: lints and compiles, does not test.

**`test:rust`** — role: test. `cargo test --workspace --locked`. Owns Rust
source, `#[test]` targets, examples, and doc tests (`cargo test` runs them).
Propagation: workspace-wide. Companions: both Clippy passes. Admission:
unmarked (minutes). In `qa`. Distinct from the built-in `effigy test`, which
plans nextest and skips doc tests. Limits: prototypes excluded; does not deny
rustdoc warnings (`docs:rust` is release-only).

**`lint:agent-tool-dispatch`**, **`test:agent-tool-dispatch`**,
**`docs:agent-tool-dispatch`** — role: scoped compile/test/rustdoc for
`crates/longhorn-agent-tool-dispatch/**` and its dependency closure. The only
per-crate Rust selectors besides `check:agent-tool-dispatch`. Companions: the
release-absence proof, `check:api-reference` (aggregated by
`check:agent-tool-dispatch`). Limits: no other crate has a scoped selector.

**`docs:rust`** — role: compile (rustdoc). `RUSTDOCFLAGS="-D warnings" cargo
doc --workspace --no-deps --locked`. Owns all Rust doc comments and crate
manifests. Admission: unmarked; release gate (`rustdoc`). Not in `qa`. Limits:
release-only; slowest of the cheap gates.

**`check:prototypes`** — role: compile. `cargo check --all-targets --locked`
in each `prototypes/*/` workspace. Owns `prototypes/*/**` (each has its own
`Cargo.lock`), plus whichever workspace crates a prototype path-depends on
(`longhorn-core`, `longhorn-update`, and the `longhorn-native-content*` set).
Propagation: a workspace crate API change can break a prototype. Companions:
`sync:prototype-locks` after a version bump. Admission: `heavy`. Release gate
(`prototypes`); deliberately absent from `qa`. Limits: `check`, not `build` --
proves the seam typechecks, not that it links.

**`release:floor`** — role: compile/lint/test at the MSRV. Runs
`scripts/check-release-floor.sh`, which reads
`release-baselines/rust-toolchains.env` and runs Clippy (default and
`--all-features`) plus `cargo test` under `rustup run`. Owns the Rust source
set, the MSRV file, and `rust-version` in `Cargo.toml`. Companions: any
workspace Rust change at a milestone; not per task. Admission: `heavy`.
Release gate (`floor`). Limits: needs the MSRV toolchain installed.

**`release:source-consumer`** — role: proof. Builds a throwaway consumer
against a git-source snapshot and asserts every probed crate resolves from
git, not path. Owns every tracked file (`git ls-files`) and crate manifests.
Companions: none. Admission: unmarked but heavyweight. Release gate (`source`).

**`check:release-gates`** — role: config consistency.
`bun scripts/check-release-gates-alignment.ts`. Owns `config/release.toml` and
`effigy.toml`. Companions: `test:release-tooling`. Admission: unmarked
(milliseconds). In `health` and `qa`.

**`test:release-tooling`** — role: test (Bun unit). Owns
`scripts/longhorn-version.ts`, `scripts/release-bump.ts`,
`scripts/check-release-gates-alignment.ts`,
`scripts/check-release-runner-tools.ts`,
`scripts/verify-private-candidate-docs-card127.ts` and their `.test.ts`,
`scripts/private-candidate-card127/**`, `Cargo.toml`, package manifests,
`docs/reference/api-surface.md`, `skills/agent-control/SKILL.md`.
Companions: `check:release-gates`. Admission: unmarked. In `qa`.

**`sync:prototype-locks`** — role: maintenance. Rewrites only Longhorn
path-package version fields in `prototypes/*/Cargo.lock` and proves each with
`cargo metadata --locked --offline`. Owns `prototypes/*/Cargo.lock` and the
workspace version. Admission: unmarked. Pre-gate step for `effigy
release:bump`; not in `[release.gates]`. Limits: never `cargo update`; refuses
third-party movement.

**`release:bump`** — role: maintenance (Rhai). Owns the workspace version, the
three npm manifests, adapter peers, `CHANGELOG.md`, the skill stamp,
`prototypes/*/Cargo.lock`, `docs/reference/api-surface.md`. Operator action,
not a selection gate.

**`release:gates`** — role: aggregate. The declared `[release.gates]` minus
`workspace`, in order: private-candidate, advisories, `docs:rust`,
`check:prototypes`, `release:floor`, `release:source-consumer`.

**`ci:rehearse`** — role: proof (clean-runner rehearsal), `heavy`. Owns
`scripts/**`, `.github/**`, `config/release.toml`, `Cargo.lock`, and every
proof input. Release-time only. Limits: not a substitute for dispatching
`release.yml`.

### TypeScript and Svelte

**`bootstrap:deps`** — role: setup. `bun install --frozen-lockfile`. Owns
`package.json`, `bun.lock`, `packages/*/package.json`. Companion: every
TypeScript gate. In `qa`.

**`check:bun-deps`** — role: precondition. Fails when
`node_modules/{typescript,svelte,@inflatable-cookie/poodle-core}` are absent.
Nested inside `check:ts` and `check:svelte`. Owns `node_modules` presence,
`package.json`, `bun.lock`. No independent entry in `qa`.

**`check:ts`** — role: typecheck. `bun x tsc -p` for each
`packages/*/tsconfig.json` (currently `longhorn`, `longhorn-poodle-svelte`,
`longhorn-tauri`). Owns `packages/*/src/**`, `packages/*/tests/**` where the
tsconfig includes them, `packages/*/tsconfig.json`, generated bindings under
`packages/*/src/**/generated/**`. Propagation: the adapter and tauri packages
resolve `@inflatable-cookie/longhorn` to workspace source, so a change in
`packages/longhorn/src` can fail them. Companions: `check:svelte`,
`test:ts`, `test:vitest`. Admission: unmarked (tens of seconds). In `qa`.
Limits: one loop over all packages; no per-package selector exists.

**`check:svelte`** — role: typecheck/lint. `bun x svelte-check` against
`packages/longhorn-poodle-svelte/tsconfig.json`. Owns
`packages/longhorn-poodle-svelte/**` (`.svelte`, `.svelte.ts`, `.ts`) and the
`packages/longhorn` sources it imports. Companions: `check:ts`. Admission:
unmarked. Limits: only the adapter package; the other two have no Svelte.

**`check:packages`** — role: packaging. `bun pm pack --dry-run
--ignore-scripts` in each `packages/*/`. Owns `packages/*/package.json`
(`files`, `exports`), `packages/*/src/**`, LICENSE. Companions:
`proof:pack-typecheck` for real resolution. Admission: unmarked. Limits:
dry-run only; proves the tarball assembles, not that it installs.

**`test:ts`** — role: test (Bun native). `scripts/test-packages.sh` runs
`packages/*/tests/**/*.test.ts` for packages without a vitest config:
`packages/longhorn/tests/**` and `packages/longhorn-tauri/tests/**`. Owns
those tests and the sources/fixtures they import. Propagation: tests import
workspace package sources. Companions: `test:vitest`, `check:ts`. Admission:
unmarked. Limits: excludes vitest-owned dirs by design.

**`test:vitest`** — role: test. `bun x vitest run` for each
`packages/*/vitest.config.ts` (currently only
`packages/longhorn-poodle-svelte`). Owns
`packages/longhorn-poodle-svelte/tests/**` (client and SSR projects) and its
sources. Companions: `test:ts`, `check:svelte`. Admission: unmarked.

### Generated bindings and cross-language

**`check:bindings`** — role: generated-artifact drift (byte-for-byte).
`cargo run -q -p longhorn-bindings -- <domain> check`, looping fifteen
domains: `bridge commands config history history-tree layout licence
native-content notifications operation settings surfaces surface-transfer
transfer update`. Owns `crates/longhorn-bindings/**` (generator), the type
definitions of the domain crates, `crates/{longhorn-core,longhorn-licence,
longhorn-update}/bindings/**`, the per-domain golden fixtures
`fixtures/<domain>/protocol-v1.json` (plus
`fixtures/surfaces/surface-bound-registered-authority-v1.json` and
`fixtures/layout/surface-bound-registered-authority-v1.json`), and the
generated TS under `packages/longhorn/src/<domain>/generated/**`. Domain→crate: `layout` reads
`longhorn-surfaces` (Card 179 folded layout in); `commands` also reads
`longhorn-command-config`; the rest map one-to-one. Propagation: a Rust type
change in a domain crate moves generated TS and conformance fixtures, and the
TypeScript that imports them. Companions: `generate:bindings` when it drifts,
`check:ts` to compile the regenerated output, the artifact proofs that rerun
per-domain binding checks. Admission: unmarked (compiles `longhorn-bindings`).
In `qa`. Limits: the domain list is hardcoded in the task, so a new
`longhorn-bindings` domain not added to the loop is silently uncovered (see
Coverage gaps).

**`generate:bindings`** — role: generator. Same domains, `write` mode. Not a
gate; the writer behind `check:bindings`.

**`check:api-reference`** — role: generated-doc drift.
`bun scripts/generate-api-reference-card126.ts` (check mode) runs `cargo
metadata --locked --no-deps`, lists every `crates/*` package and every
`packages/*/package.json` export, and diffs `docs/reference/api-surface.md`.
Owns `scripts/generate-api-reference-card126.ts`, `Cargo.toml`, crate
manifests and targets, `packages/*/package.json`, and the generated doc.
Companions: `test:release-tooling` (bump path), `proof:guides-card126`.
Admission: unmarked. In `qa`. Limits: needs `cargo metadata`.

**`generate:api-reference`** — role: generator (`--write`).

**`check:tauri-seam-strings`** — role: proof (cross-language string parity).
`bun scripts/verify-tauri-seam-strings.ts`. Owns
`crates/longhorn-tauri-*/src/**/*.rs` (`#[tauri::command]` names, `longhorn://`
events) and `packages/longhorn-tauri/src/**`. Companions: `host-protocol`.
Admission: unmarked. In `qa`. Limits: only the `longhorn-tauri` port files;
allow-lists named crates/ports by hand.

### Repository checks

**`check:runner-tools`** — role: scan. `bun scripts/check-runner-tools.ts`.
Scans `scripts/**`, `.github/**`, `config/**` (`.ts .tsx .js .mjs .cjs .sh
.yml .yaml .toml`), `effigy.toml`, and `config/release.toml` for forbidden
runner tools (`rg`, `ripgrep`), and checks `.github/workflows/release.yml`
installs every tool a release gate needs. Companions:
`test:release-tooling`. Admission: unmarked (seconds). In `health` and `qa`.

**`check:consumer-isolation`** — role: scan (containment). Owns `crates/**`,
`packages/**`, `scripts/**`, `examples/**`, `prototypes/**`, `fixtures/**`
(`.ts .tsx .svelte .rs .json .toml .sh`). Fails a `*_REPO` override, an
absolute path into another checkout, or `resolve(repoRoot, "../name")`.
Companions: `check:repo-containment`. Admission: unmarked. In `qa`.

**`check:repo-containment`** — role: scan (containment). Same scanned dirs
plus root `Cargo.toml` and `package.json`. Fails Cargo `path`, package
`file:`/`link:`, and `join(repoRoot, "../…")` that leave the tree.
Companions: `check:consumer-isolation`. Admission: unmarked. In `qa`.

**`check:agent-control-release-absence`** — role: compile + byte scan (proof).
Builds `longhorn-tauri-agent-control` in three feature states (`off`,
`agent-control`, `agent-control,evaluate`) into isolated target dirs and scans
rlibs for core-crate and shim markers, with a positive control. Owns
`crates/longhorn-tauri-agent-control/**`,
`crates/longhorn-agent-control/src/lib.rs`,
`packages/longhorn/src/agent-control/inject.ts` markers, `Cargo.toml` /
`Cargo.lock`. Companions: `check:agent-control-shim`. Admission: unmarked but
builds; in `qa`. Limits: macOS/Tauri build; heavy in practice.

**`check:agent-control-shim`** — role: generated-artifact drift. Bundles
`packages/longhorn/src/agent-control/inject.ts` as an IIFE and diffs the
committed `crates/longhorn-tauri-agent-control/src/agent_control_shim.js`.
Companions: `check:agent-control-release-absence`. Admission: unmarked. In
`qa`. Limits: bundle + byte compare, no test.

**`generate:agent-control-shim`** — role: generator (`--write`).

**`check:agent-tool-dispatch-release-absence`** — role: proof (graph + byte
scan). Reads `cargo tree -p longhorn-agent-tool-dispatch` and builds its rlib
with `--no-default-features`, asserting contract 022 symbols are absent, with
`crates/longhorn-agent-control/src/lib.rs` as positive control. Owns
`crates/longhorn-agent-tool-dispatch/**`, the control file, `Cargo.lock`.
Companions: `check:agent-tool-dispatch`. Admission: unmarked but builds; in
`qa`.

**`check:agent-tool-dispatch`** — role: aggregate (not in `qa`). Runs
`lint:agent-tool-dispatch`, `test:agent-tool-dispatch`,
`docs:agent-tool-dispatch`, `check:agent-tool-dispatch-release-absence`,
`check:api-reference`.

**`check:agent-control-skill`** — role: drift lock + fixtures. Owns
`crates/longhorn-agent-control/src/tools.rs` (`CONTROL_TOOL_NAMES`),
`Cargo.toml` (workspace version), `skills/agent-control/SKILL.md`,
`skills/agent-control/scripts/find-instance.ts`,
`scripts/install-agent-control-skill.ts`. Companions: none. Admission:
unmarked. In `health` and `qa`.

**`check:bun-links`** — role: scan (machine state). Reports registered global
bun links and fails when a workspace dependency resolves through a link that
leaves the repository. Owns the machine's `~/.bun/install/global/node_modules`
and this tree's `node_modules` resolution. Admission: unmarked. In
`ci:rehearse` only. Limits: machine state, not tree state; a clean runner
makes it a no-op.

**`agent-control:install-skill`** — role: maintenance (Rhai wrapper over
`scripts/install-agent-control-skill.ts`). Copies `skills/agent-control/` into
a consumer repo. Operator-invoked; no consumer-app writes from this repo.

**`proof:agent-tool-dispatch-source-consumer`** — role: proof (disposable
source consumer). Requires `AGENT_LONGHORN_REPO` and `AGENT_PROJECTS_ROOT`,
stages a `mktemp` fixture from exact clean SHAs, and compiles outside both
repos. Owns `fixtures/agent-tool-dispatch-provider-free`,
`crates/longhorn-agent-tool-dispatch/**`. Not in `qa`; acceptance evidence,
not a standing gate.

### Documentation

**`qa:docs`** — role: docs aggregate. Members below.

**`qa:docs:links`** — role: docs. Link check on `README.md`, `AGENTS.md`,
`docs/README.md`, the knowledge indexes, `docs/guides/README.md`,
`docs/guides/getting-started.md`, `docs/reference/README.md`.

**`qa:docs:agent-defaults`** — role: docs scan. Forbids `--repo .` in
`AGENTS.md`, `README.md`, `docs/README.md`.

**`qa:docs:paths`** — role: docs. Requires `README.md`, `AGENTS.md`,
`scripts/README.md`, `docs/README.md`, and every `docs/knowledge*` index,
architecture and contract file to exist.

**`qa:docs:catalog-links`** — role: docs. `effigy docs check links` over
`README.md` and all Markdown under `docs/`. This is the check that resolves
links in a new knowledge file.

**`held-surface`** — role: docs proof. Owns `docs/reference/held-surface.md`,
`docs/reference/api-surface.md`, `docs/guides/package-selection.md`.
Companions: `check:api-reference`. Limits: parses a fixed register table.

**`host-protocol`** — role: cross-language proof. Reads
`crates/longhorn-tauri-*/src/**/*.rs`, the `examples/permissions` and
`examples/capabilities` JSON under those crates, and `packages/*/src/**/*.ts`;
asserts every invoke/event name has a counterpart or a documented seam.
Companions: `check:tauri-seam-strings`. Limits: string inventory, not types.

### Artifact proofs

`proof:artifacts` is `admission = "heavy"`, in `qa`, and runs fourteen scripts
in order. Each packs the TypeScript packages (`bun pm pack`) and/or copies the
named Rust crates into a `mkdtemp` stage, installs the pinned registry Poodle
release, and typechecks/builds/tests the stage. They all read
`bun.lock`/`package.json`, `scripts/proof-install.ts`, `scripts/poodle-
release.ts`, and `scripts/longhorn-version.ts`, and none may write a consumer
repo.

| Proof | Owned tree inputs | Proves |
| --- | --- | --- |
| `verify-app-shell-proof.ts` | `examples/app-shell-proof/{split-shell,nucleus,loophole,common}/**`; `packages/longhorn`, `packages/longhorn-poodle-svelte` | composed shell mounts against both host adapters |
| `verify-bridge-topology-conformance.ts` | `examples/bridge-topology-proof/**`; `crates/longhorn-bridge/{Cargo.toml,src}/**`; `crates/longhorn-tauri-bridge/{src,examples/capabilities/query-only.json}`; `packages/longhorn/src/bridge/**` | each topology example imports only its declared surface, no production edge |
| `verify-bridge-topology-artifacts.ts` | `examples/bridge-topology-proof/**` (`declarations.json`); packs `longhorn-core`, `longhorn-bridge`, `longhorn-tauri-bridge` | the five topologies install and run as isolated graphs |
| `verify-settings-composition-proof.ts` | `examples/settings-composition-proof/**`; `fixtures/{config,settings}/protocol-v1.json`; packs `longhorn-{core,config,settings,settings-config,tauri-settings,tauri-config}` | settings composition against the real registry |
| `verify-command-system-artifacts.ts` | `examples/command-system-proof/**`; packs `longhorn-{core,config,settings,command,command-config,command-settings,tauri-command}`; runs `longhorn-bindings commands check` | command system packed and installed |
| `verify-history-system-artifacts.ts` | `examples/history-system-proof/**`; packs `longhorn-{core,history,tauri-history}` | linear history packed and installed |
| `verify-history-tree-artifacts.ts` | `examples/history-tree-artifact-proof/**`; packs `longhorn-{core,history,history-tree,tauri-history-tree}` | fork history packed and installed |
| `verify-operation-notification-artifacts.ts` | `examples/operation-notification-proof/**`; packs `longhorn-{core,bridge,operation,notifications,tauri-operation,tauri-notifications}`; runs `longhorn-bindings operation,notifications check` | operations and notifications together |
| `verify-native-content-artifacts.ts` | `examples/native-content-system-proof/**`, the three `examples/tauri-native-content-*-proof/**`; `prototypes/native-content/**`; `fixtures/native-content/**`; packs `longhorn-{core,native-content,tauri-native-content-child-view,native-content-isolated-window,native-content-backing-surface}` | native content across the three Tauri mechanisms |
| `verify-poodle-preview.ts` | `bun.lock`, installed Poodle in `node_modules`, `packages/longhorn-poodle-svelte/package.json` peer range | the mounted Poodle artifacts match the lock; Svelte peer range `>=5.56.8 <6` |
| `verify-greenfield-card125.ts` | `examples/greenfield-compositions/**`; `fixtures/greenfield/card125/composition-matrix-v1.json`; all three TS packages; 24 Rust crates; `release-baselines/rust-toolchains.env` via `msrv.ts` | greenfield compositions carry no donor vocabulary; MSRV manifests resolve |
| `verify-guides-card126.ts` | `docs/guides/*.md`, `docs/reference/{README,api-surface}.md`, `docs/README.md`, `README.md`, `examples/greenfield-compositions/README.md`; `crates/*` and `packages/*` directory counts; `scripts/generate-api-reference-card126.ts` | guides match the generated API surface and local links resolve |
| `verify-documented-commands.ts` | `effigy.toml`; every `examples/**/*.md` | every `effigy <task>` named in an example README resolves |
| `verify-pack-typecheck.ts` | `packages/longhorn`, `packages/longhorn-poodle-svelte`; root `package.json` dev pins; `bun.lock`; registry Poodle at newest and floor | the packed adapter typechecks against registry-only Poodle |

`proof:pack-typecheck` is also its own selector (same script). Limits across
all proofs: they install from the lock against the registry and network, and
take minutes; nothing excludes them from the board -- `proof:artifacts` is in
`qa`.

## Cross-cutting input classes

### Rust crate dependents

`cargo test/clippy --workspace` is all-or-nothing. Only
`longhorn-agent-tool-dispatch` has scoped selectors. Everything else needs the
whole Rust lane: `lint:rust`, `lint:rust:features`, `test:rust`. A shared
crate (`longhorn-core` feeds 40+ manifests) and a leaf crate
(`longhorn-credential-keyring` has no in-tree dependents) select the same
three. `check:prototypes` additionally applies when a prototype
path-depends on the changed crate. Evidence missing for narrowing: no
per-crate `test:rust:<crate>` selector exists, and `cargo` alone is not an
admissible Effigy selector.

### TypeScript package dependents

`check:ts` loops all packages, so any package change selects all three
typechecks. The adapter depends on `packages/longhorn`; `longhorn-tauri` does
too. `check:svelte` applies only to a change that reaches
`packages/longhorn-poodle-svelte` source or its imports. Tests split:
`packages/longhorn` and `packages/longhorn-tauri` tests are `test:ts`;
`packages/longhorn-poodle-svelte` tests are `test:vitest`. `check:packages`
applies to any package manifest or source change. No per-package selector
exists.

### Rust types crossing into TypeScript

A change to a domain crate's public types is three things at once:

1. Rust: `lint:rust`, `lint:rust:features`, `test:rust`.
2. Generated drift: `check:bindings` for that domain, then
   `generate:bindings` and `check:ts` for the regenerated
   `packages/longhorn/src/<domain>/generated/**`.
3. Proofs that re-run the domain's binding check or consume the types:
   `proof:artifacts` for the matching proof, and `check:prototypes` when a
   prototype consumes the crate.

`check:bindings` is the only gate that proves the generator input moved; it
does not prove the TypeScript compiles, so `check:ts` is always a companion.

### Docs-only edits

Select by which documentation surface changed:

- Any Markdown under `docs/` or `README.md`/`AGENTS.md`:
  `qa:docs` (`links`, `catalog-links`, `paths`).
- `docs/reference/{held-surface,api-surface}.md` or `docs/guides/package-selection.md`:
  add `held-surface`; `proof:guides-card126` when `api-surface.md` moved.
- Any `docs/guides/*.md`, `docs/reference/README.md`, `docs/README.md`,
  `README.md`, or `examples/greenfield-compositions/README.md`:
  `proof:guides-card126`.
- `examples/**/*.md`: `proof:documented-commands`.
- Knowledge files: `qa:docs:paths` covers the index/contract set only.

No Rust or TypeScript suite is required for a docs-only change.

### Proof generators, fixtures and examples

`scripts/verify-*.ts`, `scripts/*-proof/**`, `examples/**`, and `fixtures/**`
select `proof:artifacts` (heavy), plus `check:consumer-isolation` and
`check:repo-containment`, which scan those trees. `examples/*/src-tauri/**`
and `examples/*/rust/**` are also workspace members, so they select the Rust
lane. A fixture consumed by a Bun test also selects `test:ts` or
`test:vitest`. The golden `fixtures/<domain>/protocol-v1.json` files are an
exception: they are `check:bindings` inputs, not proof inputs.

### Added, deleted, renamed files

- New crate: add to the root workspace `members`; selects the full Rust lane
  and `fmt:rust` (member list), plus `check:repo-containment` (root manifest).
- New package: `check:ts`, `check:packages`, `test:ts`/`test:vitest`,
  `check:consumer-isolation`, `check:repo-containment`,
  `proof:pack-typecheck` only if it is a pack target.
- New `longhorn-bindings` domain: `check:bindings` will not see it until the
  task loop in `effigy.toml` names it -- a real coverage gap.
- New `longhorn-tauri-*` crate: `check:tauri-seam-strings` and `host-protocol`
  read `crates/longhorn-tauri-*`, so they pick it up automatically.
- Rename a file inside a crate/package: no selector changes; the content
  gates follow the new path.
- Delete: the selectors above; `check:repo-containment` guards manifests that
  still name a path.

### Staged, unstaged, relevant untracked inputs

Every gate reads the working tree, not the Git index: `--locked` compares
manifests to `Cargo.lock`/`bun.lock`, and the scan selectors walk the files.
Staged vs unstaged does not change the answer. Untracked files under a scanned
tree are seen (the scan selectors walk directories; the graph's
`git status --porcelain` reports `??`). Untracked files that no manifest or
glob reaches -- an unregistered crate, a new package not in `workspaces` --
select nothing until wired. Ignored paths (`target/`, `node_modules/`,
`.effigy/`, `.svelte-kit/`) are never selection inputs.

## Opaque and global inputs

| Input | Safe selection | Why |
| --- | --- | --- |
| `Cargo.lock` | conservative: full Rust lane + `check:prototypes` + `release:floor` | no selector proves the lock alone; every `--locked` gate fails on drift, and prototype locks are separate |
| `bun.lock` | conservative: `bootstrap:deps`, full TypeScript lane, `proof:artifacts` | proofs install from the lock; `proof:poodle-preview` and `proof:pack-typecheck` verify Poodle sha512/peer range against it |
| `package.json` (root) | `bootstrap:deps`, `check:ts`, `check:svelte`, `proof:artifacts` | dev pins and peer ranges |
| `effigy.toml` (tasks, includes) | conservative: full board + `check:release-gates` + `check:runner-tools` + `test:release-tooling` + `proof:documented-commands` | changing tasks changes selection itself; no selector validates selection |
| `config/release.toml` | `check:release-gates`, `test:release-tooling`, private-candidate proof, `check:runner-tools` | release-gate alignment and runner-tool mapping |
| `rust-toolchain.toml` | conservative: `lint:rust`, `lint:rust:features`, `test:rust`, `check:prototypes` | no selector reads the pinned stable channel; workflows parse it |
| `release-baselines/rust-toolchains.env` | `release:floor`, `proof:artifacts` | MSRV gates and `msrv.ts` manifest generation |
| `.github/workflows/**` | `check:runner-tools`, `ci:rehearse` | install-step mapping and clean-runner rehearsal |
| generated `.ts` under `packages/*/src/**/generated/**` | `check:bindings`, `check:ts` | drift vs compile |
| `crates/*/bindings/**` | `check:bindings` | fixtures for conformance suites |
| `target/**`, `node_modules/**`, `.effigy/**` | none | ignored build/cache state |

Anything not in this table and not matched by a selector above is unresolved:
declare it broad and let the planner judge, never treat it as no checks.

## Representative synthetic change sets

| # | Change | Expected selectors | Reason |
| --- | --- | --- | --- |
| 1 | `docs/guides/getting-started.md` prose edit | `qa:docs`, `proof:guides-card126` | catalogue/link checks plus the guide-content proof |
| 2 | leaf Rust crate: `crates/longhorn-credential-keyring/src/**` | `fmt:rust`, `lint:rust`, `lint:rust:features`, `test:rust`, `check:consumer-isolation`, `check:repo-containment` | no per-crate selector; no in-tree dependents but the lane is workspace-wide |
| 3 | shared Rust type: `crates/longhorn-history/src/**` | Rust lane + `check:bindings` + `check:ts` + `test:ts` + `test:vitest` + `proof:artifacts` + `check:prototypes` | history is a bindings domain; `longhorn-history` feeds `longhorn-bindings`, `history-tree`, `tauri-history`, `prototypes/history-tree` |
| 4 | leaf TS package: `packages/longhorn-tauri/src/transport/**` | `check:ts`, `test:ts`, `check:packages`, `check:tauri-seam-strings`, `host-protocol` | no dependents; typecheck + its tests + seam/protocol scans |
| 5 | proof generator/fixture: `scripts/verify-history-tree-artifacts.ts` or `fixtures/greenfield/card125/composition-matrix-v1.json` | `proof:artifacts`, `check:consumer-isolation`, `check:repo-containment`, plus `test:ts` if a Bun test reads the fixture, plus `check:bindings` if the fixture is a golden `fixtures/<domain>/protocol-v1.json` | proof scripts and fixtures are the proof inputs; scan selectors walk both trees |
| 6 | opaque: `Cargo.lock` | conservative Rust lane + `check:prototypes` + `release:floor` | no lock-only selector; every `--locked` gate depends on it |
| 6b | opaque config: `config/release.toml` | `check:release-gates`, `test:release-tooling`, private-candidate proof, `check:runner-tools` | alignment, tool mapping, candidate facts |

Case 3 expansion, for review: `longhorn-history` is read by
`longhorn-bindings` (domain `history`), `longhorn-history-tree`,
`longhorn-tauri-history`, and `prototypes/history-tree`. A type change must
regenerate `packages/longhorn/src/history/generated/**`, recompile the
TypeScript that imports it, and rerun the linear-history and fork-history
proofs. Missing any of those is a silent break.

## Coverage gaps and unresolved inputs

- No per-crate Rust selector except `longhorn-agent-tool-dispatch`. A leaf and
  a shared crate select the same three-lane board. Missing evidence: an
  Effigy cargo package filter expressed as a selector, or per-crate
  `test:rust`/`lint:rust` tasks.
- No per-package TypeScript selector. `check:ts` loops all packages.
- `check:bindings`'s domain list is hardcoded in `effigy.toml`. A new
  generator domain is uncovered until the loop is edited. No selector detects
  that.
- `check:svelte` hardcodes `packages/longhorn-poodle-svelte/tsconfig.json`; a
  second Svelte package would be uncovered.
- Generated and ignored paths are invisible to Effigy's graph, so `graph
  affected` cannot prove `packages/*/src/**/generated/**`, `crates/*/bindings
  /**`, or lockfiles are covered. The honest answer for those inputs is the
  declared selector, not a graph result.
- No selector validates `rust-toolchain.toml`, `effigy.toml` task
  definitions, or `.github/workflows/**` beyond `check:runner-tools`; those
  stay conservative.
- `proof:artifacts` is one heavy step. There is no per-proof admission, so a
  one-fixture change cannot select a single proof through admission today.
- Prototype locks (`prototypes/*/Cargo.lock`) and the root lock are separate;
  `sync:prototype-locks` is the only writer and is not a gate.
- Untracked files outside a manifest/glob reach select nothing. That is
  unresolved, not "no checks".

## Evidence and limits

Read to build this map: `effigy.toml`, `config/release.toml`,
`scripts/README.md`, the `scripts/verify-*` and proof scripts named above,
`crates/longhorn-bindings/{Cargo.toml,src}` and its domain list, each
`packages/*/package.json` and `tsconfig.json`/`vitest.config.ts`, the root
`Cargo.toml`, `.github/workflows/{ci,release}.yml`,
`docs/knowledge/contracts/release.md`, and Effigy guides 076 (code graph and
agent workflows) and 080 (host-wide validation admission).

This map records what the commands read and what they do not. It was not built
by running the board or measuring timings; cost figures come from the comments
Longhorn already keeps next to those tasks. Independent review should check
each entry against the manifest/script it names and walk the six synthetic
change sets on paper.
