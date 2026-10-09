# Validation Input Map

Status: active first pass  
Owner: Tom  
Updated: 2026-10-09
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

## Bounded QA groups

Maintained groups live in `[qa.groups]` in `effigy.toml`. Their contract is
[Effigy 051](https://github.com/inflatable-cookie/effigy/blob/7b11c039d75519fffb1a770eaa61ca62097665e5/docs/knowledge/contracts/051-bounded-qa-groups-contract.md)
and the worker workflow is [Effigy guide
081](https://github.com/inflatable-cookie/effigy/blob/7b11c039d75519fffb1a770eaa61ca62097665e5/docs/guides/081-bounded-qa-groups-workflow.md).
They compose existing selectors; they do not infer scope or filter members.
An unmapped input and every declared coverage gap return `needs_planner`
before execution. The existing `qa`, `qa:docs`, `proof:artifacts`, CI and
release gates keep their current members and meaning.

| Group | Declared work | Boundary |
| --- | --- | --- |
| `longhorn-docs` | `qa:docs:links`, `qa:docs:agent-defaults`, `qa:docs:paths`, `qa:docs:catalog-links` | General links, index paths and agent defaults only; omits `held-surface` and `host-protocol`. |
| `longhorn-getting-started-docs` | The two docs link selectors plus `proof:guides-card126` | The fixed guide corpus, its links and generated API inventory; other artifact proofs remain out. |
| `longhorn-bridge-topology-conformance` | `proof:bridge-topology-conformance` | Five declared bridge compositions, bridge crate sources, the Longhorn bridge package scope, and the query-only capability. |
| `longhorn-pack-typecheck` | `proof:pack-typecheck` | Packed scoped Longhorn and adapter packages, their package paths, locked registry dependencies, and declared installed Poodle metadata. |
| `longhorn-poodle-preview` | `proof:poodle-preview` | Poodle lock/version/integrity metadata and the installed peer range; no Longhorn package typecheck. |
| `longhorn-documented-commands` | `proof:documented-commands` | Effigy command names in Markdown under `examples/`; task bodies are not validated. |
| `agent-tool-dispatch` | `check:agent-tool-dispatch` | Package obligations plus its workspace-wide API-reference inventory check; not a general workspace or lockfile group. |
| `longhorn-ts` | Package selectors for `longhorn` plus its Tauri and Poodle-Svelte consumers | Typechecks and tests the source package and both workspace consumers. |
| `longhorn-tauri-ts` | `check:ts:longhorn-tauri`, `test:ts:longhorn-tauri` | The Tauri package only. |
| `longhorn-poodle-svelte-ts` | Its TypeScript check, `check:svelte`, and package Vitest selector | The Poodle-Svelte package only. |
| `longhorn-bindings` | `check:bindings` | All fifteen registered domains; the generator's compile-closure crates are gaps on both `cargo-package:` and `path:crates/<crate>/**` tokens, plus the opaque `input:bindings-generator-transitive-compile-dependencies` token. |
| `agent-control-absence` | `check:agent-control-release-absence`, `check:agent-control-shim` | Compile/absence and generated-shim checks; the five release-absence compile crates are gaps on both `cargo-package:` and `path:crates/<crate>/**` tokens, plus the opaque `input:agent-control-marker-source-coverage` token. |

Expected runtime is declared where the owner has a basis. For example,
`agent-control-absence` declares `expected_wall_ms = 300000`; groups without an
expectation report unknown. Run records separate admission wait from execution
duration. An accepted plan resolves every declared member regardless of
which covered scope token was supplied.

### Task-table-only Effigy edits

An `effigy.toml` edit is task-table-only only when it changes one `[tasks]`
selector entry or one `[tasks."<selector>"]` table and leaves `qa`/`ci`
membership, includes, `[qa.groups]`, admission, and toolchain settings
untouched. The opaque token below asserts that classification and names the
edited selector; Effigy does not inspect the diff. The group must contain that
selector. Do not add `path:effigy.toml` for these cases: that token represents a
global or otherwise unclassified manifest change and remains
`needs_planner`, even when paired with a task-table token. Other task-table
entries without a listed route also remain `needs_planner`.

| Task-table scope tokens | Group | Selector the group runs |
| --- | --- | --- |
| `input:effigy-task-table-proof-pack-typecheck`, `external:root-node-modules`, `external:bun-runtime` | `longhorn-pack-typecheck` | `proof:pack-typecheck` |
| `input:effigy-task-table-proof-bridge-topology-conformance`, `external:root-node-modules`, `external:bun-runtime` | `longhorn-bridge-topology-conformance` | `proof:bridge-topology-conformance` |
| `input:effigy-task-table-proof-poodle-preview`, `external:root-node-modules`, `external:bun-runtime` | `longhorn-poodle-preview` | `proof:poodle-preview` |
| `input:effigy-task-table-proof-documented-commands` | `longhorn-documented-commands` | `proof:documented-commands` |
| `input:effigy-task-table-proof-guides-card126` | `longhorn-getting-started-docs` | `proof:guides-card126` |

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
`Cargo.lock`) and every workspace crate a prototype path-depends on, directly
or through a local prototype crate. The edges:

- `prototypes/agent-control/` — none (it depends on no workspace crate).
- `prototypes/gpui-composition/` — full closure through the local
  `longhorn-gpui-windowing-prototype`: core, config, display, gpui-windowing,
  licence, notifications, operation, poodle, settings, surfaces,
  surfaces-config, transfer, update, url, windowing, windowing-config.
- `prototypes/gpui-windowing/` — the same closure: core, config, display,
  gpui-windowing, licence, notifications, operation, poodle, settings,
  surfaces, surfaces-config, transfer, update, url, windowing,
  windowing-config.
- `prototypes/history-tree/` — core, history.
- `prototypes/native-content/` — core.
- `prototypes/native-content-backing-surface/` — core.
- `prototypes/native-content-child-webview/` — core.
- `prototypes/native-content-isolated-window/` — core, display, windowing
  (display arrives through windowing).

Union: `longhorn-{config,core,display,gpui-windowing,history,licence,notifications,operation,poodle,settings,surfaces,surfaces-config,transfer,update,url,windowing,windowing-config}`.
Transitive edges that are easy to miss: gpui-windowing and windowing pull
`longhorn-display`; poodle pulls `longhorn-settings`; transfer pulls
`longhorn-surfaces` and `longhorn-surfaces-config`; licence and update pull
`longhorn-url`. The workspace `longhorn-native-content*` crates are not
prototype dependencies; the native-content prototypes use the prototype-local
`longhorn-native-content-prototype` and `longhorn-core` only. Propagation: a
change to any crate in that union selects this release-only aggregate even
when no prototype file moves.

Companions: `sync:prototype-locks` after a version bump. Admission: `heavy`.
Release gate (`prototypes`); deliberately absent from `qa`. Limits: `check`,
not `build` -- proves the seam typechecks, not that it links.

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
`scripts/qa-group-routing.test.ts` (which plans each maintained route with
Effigy and asserts accepted/refused scope dispositions),
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
`check:prototypes`, `release:floor`, `release:source-consumer`. The first two
have no Effigy selector of their own; dispatch them only through this
aggregate.

**`private-candidate`** — role: docs fact-check (release gate, reachable only
through `release:gates`). Runs
`bun scripts/verify-private-candidate-docs-card127.ts`. Owns
`fixtures/release/card127/private-0-1-candidate-v1.json` (a receipt it reads
and checks), `docs/reference/private-0-1-candidate.md`,
`docs/guides/compatibility-and-upgrades.md`, `CHANGELOG.md`,
`config/release.toml`, and `scripts/private-candidate-card127/support.ts`. It
asserts the receipt's version, commit, set-hash, protocol, graph and audit
facts appear in the prose. Companions: `test:release-tooling` (tests the
parser). Admission: unmarked, release-time only. Limits: no narrow selector;
the receipt is a fixture, so the generic fixtures rule must not route it to
`proof:artifacts` -- that aggregate does not run this check.

**`advisories`** — role: supply-chain scan (release gate, reachable only
through `release:gates`). Runs `cargo deny check advisories`. Owns `deny.toml`
(the dated allowances), `Cargo.toml`/`Cargo.lock` (the resolved graph), and
the external `cargo-deny` tool plus the advisory database it fetches.
Companions: `check:runner-tools` (`.github/workflows/release.yml` must install
cargo-deny before `release:gates`). Admission: unmarked but network-bound;
release-time only. Limits: no in-repo selector; the advisory database and tool
version are external state, so a local pass is not reproducible evidence.

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
The aggregate remains unchanged. `check:ts:longhorn`,
`check:ts:longhorn-tauri`, and `check:ts:longhorn-poodle-svelte` run the same
check for one package after `check:bun-deps`; they are the package-check
members used by the bounded groups.

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
unmarked. Limits: excludes vitest-owned dirs by design. With no argument, the
aggregate still runs both Bun-native suites. `test:ts:longhorn` and
`test:ts:longhorn-tauri` pass one package directory to the existing script;
`test:ts:longhorn-poodle-svelte` selects its existing Vitest config. These
package selectors are bounded group building blocks.

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
transfer update`. Owns `crates/longhorn-bindings/**` (generator),
`crates/longhorn-core/**` (the shared declarations and constants every domain
renderer reads — `crates/longhorn-bindings/Cargo.toml` enables core's
`bindings` feature, and `store_compatibility.rs` exports `CompatibilityStore`
under `#[cfg_attr(feature = "bindings", ts(export))]`), the full source of
the fifteen domain crates, not only their type definitions — the generator
executes domain behaviour too: `crates/longhorn-bindings/src/licence.rs` calls
`longhorn_licence::key_conformance_cases()`
(`crates/longhorn-licence/src/key.rs`), which drives `LicenceKey::parse` and
emits `packages/longhorn/src/licence/generated/key-conformance.json`, so a
parser or conformance-case change moves generated output with no public type
change — the per-domain golden fixtures
`fixtures/<domain>/protocol-v1.json`, the layout conformance outputs
`fixtures/layout/{surface-bound,window-bound}-conformance-v1.json`, and the
generated output under `packages/longhorn/src/<domain>/generated/**` (TS plus
`key-conformance.json`). Domain→crate: `layout` reads
`longhorn-surfaces` (Card 179 folded layout in); `commands` also reads
`longhorn-command-config`; every domain also reads `longhorn-core`
(`HistoryId`/`HistoryRevision`/`MAX_OPAQUE_ID_BYTES` in `history.rs`,
`WindowId`/`SurfaceId` in `surfaces.rs`, and core ids elsewhere); the rest map
one-to-one. Propagation: a `longhorn-core` declaration, constant or `#[ts]`
annotation change moves generated output in the affected domains exactly as a
domain-crate change does, so a core-only edit selects `check:bindings` plus
`check:ts` and the `test:ts`/`test:vitest` imports; a behavioural licence
change does the same through the generated JSON. Companions: `generate:bindings` when it drifts,
`check:ts` to compile the regenerated output, `test:ts`/`test:vitest` where a
Bun or Svelte test imports the same fixture, and the artifact proofs that
rerun per-domain binding checks. Admission: unmarked (compiles
`longhorn-bindings`). In `qa`. Limits: the domain list is hardcoded in the
task, so a new `longhorn-bindings` domain not added to the loop is silently
uncovered. The committed per-type files under `crates/*/bindings/**` (ts-rs
`#[ts(export)]` output) are not read or diffed by this selector or any other;
see Coverage gaps. The two `surface-bound-registered-authority-v1.json` files
are `#[cfg(test)]` generator inputs, so `test:rust` reads them, not this
gate.

The `longhorn-bindings` group runs this selector. Its gaps fire on ordinary
tokens: every crate in the generator's compile closure (derived from `cargo
metadata`; includes `longhorn-url` and `longhorn-surfaces-config`) is declared
as both a `cargo-package:` and a `path:crates/<crate>/**` gap, so a scope token
for any of them returns `needs_planner`, as does the opaque
`input:bindings-generator-transitive-compile-dependencies` token.

**`generate:bindings`** — role: generator. Same domains, `write` mode. Not a
gate; the writer behind `check:bindings`.

**`check:api-reference`** — role: generated-doc drift.
`bun scripts/generate-api-reference-card126.ts` (check mode) runs `cargo
metadata --locked --no-deps`, lists every `crates/*` package and every
`packages/*/package.json` export, and diffs `docs/reference/api-surface.md`.
Owns `scripts/generate-api-reference-card126.ts`, `Cargo.toml`, crate
manifests and targets, `packages/*/package.json`, and the generated doc.
Companions: `test:release-tooling` (bump path), `proof:artifacts` (member
`scripts/verify-guides-card126.ts`).
Admission: unmarked. In `qa`. Limits: needs `cargo metadata`.

**`generate:api-reference`** — role: generator (`--write`).

**`check:tauri-seam-strings`** — role: proof (cross-language string parity).
Runs `scripts/verify-tauri-seam-strings.ts`. Owns
`crates/longhorn-tauri-*/src/**/*.rs` (`#[tauri::command]` names, `longhorn://`
events) and `packages/longhorn-tauri/src/**` — `src` only, unlike the
recursive `host-protocol` scan. Companions: `host-protocol`.
Admission: unmarked. In `qa`. Limits: only the `longhorn-tauri` port files;
allow-lists named crates/ports by hand.

### Repository checks

**`check:runner-tools`** — role: scan. `bun scripts/check-runner-tools.ts`.
Scans `scripts/**`, `.github/**`, `config/**` (`.ts .tsx .js .mjs .cjs .sh
.yml .yaml .toml`), `effigy.toml`, and `config/release.toml` for forbidden
runner tools (`rg`, `ripgrep`), and checks `.github/workflows/release.yml`
installs every tool a release gate needs. Companions:
`test:release-tooling`. Admission: unmarked (seconds). In `health` and `qa`.

**`check:consumer-isolation`** — role: scan (containment). Runs
`scripts/verify-consumer-isolation.ts`. Owns `crates/**`,
`packages/**`, `scripts/**`, `examples/**`, `prototypes/**`, `fixtures/**`
(`.ts .tsx .svelte .rs .json .toml .sh`). Fails a `*_REPO` override, an
absolute path into another checkout, or `resolve(repoRoot, "../name")`.
Companions: `check:repo-containment`. Admission: unmarked. In `qa`.

**`check:repo-containment`** — role: scan (containment). Runs
`scripts/verify-repo-containment.ts`. Same scanned dirs
plus root `Cargo.toml` and `package.json`. Fails Cargo `path`, package
`file:`/`link:`, and `join(repoRoot, "../…")` that leave the tree.
Companions: `check:consumer-isolation`. Admission: unmarked. In `qa`. Limits:
it tests escape, not existence -- a named path inside the tree need not exist.

**`check:agent-control-release-absence`** — role: compile + byte scan (proof).
Runs `scripts/verify-agent-control-release-absence.ts`. Builds
`longhorn-tauri-agent-control` in three feature states (`off`,
`agent-control`, `agent-control,agent-control-evaluate`) into isolated target
dirs and scans rlibs for core-crate and shim markers, with a positive control.
Owns `crates/longhorn-tauri-agent-control/**` (source and manifest),
`crates/longhorn-agent-control/**` (source and manifest — the evaluate marker
and `#[cfg(feature = "agent-control-evaluate")]` helper are in
`src/server/mcp.rs`; `src/lib.rs` is not the whole input), the shim bundle
source `packages/longhorn/src/agent-control/{inject,shim}.ts` (markers),
`Cargo.toml` / `Cargo.lock`. Companions: `check:agent-control-shim`.
Admission: unmarked but builds; in `qa`. Limits: macOS/Tauri build; heavy in
practice.

The `agent-control-absence` group runs this proof with
`check:agent-control-shim`. Its gaps fire on ordinary tokens: the five
release-absence compile crates (`longhorn-agent-control`,
`longhorn-tauri-agent-control`, `longhorn-core`, `longhorn-config`,
`longhorn-tauri-config`) are declared as both `cargo-package:` and
`path:crates/<crate>/**` gaps, so a scope token for any of them returns
`needs_planner`, as does the opaque
`input:agent-control-marker-source-coverage` token.

**`check:agent-control-shim`** — role: generated-artifact drift. Bundles the
injection entry `packages/longhorn/src/agent-control/inject.ts` with
`Bun.build` as an IIFE and diffs the committed
`crates/longhorn-tauri-agent-control/src/agent_control_shim.js`. The bundle
closure is `inject.ts` plus the implementation module it imports,
`packages/longhorn/src/agent-control/shim.ts` (which imports nothing else);
the writer is `scripts/agent-control-shim.ts`. Owns those three files, the
generated JS, and the Bun bundler/TypeScript toolchain. Companions:
`check:agent-control-release-absence`. Admission: unmarked. In `qa`. Limits:
bundle + byte compare, no test. A `shim.ts`-only edit is invisible to
`check:ts`/`test:ts` for the committed asset and does not select this gate by
an ordinary TS rule; it must be declared as a shim-closure input.

**`generate:agent-control-shim`** — role: generator (`--write`).

**`check:agent-tool-dispatch-release-absence`** — role: proof (graph + byte
scan). Runs `scripts/verify-agent-tool-dispatch-release-absence.ts`. Reads
`cargo tree -p longhorn-agent-tool-dispatch` and builds its rlib
with `--no-default-features`, asserting contract 022 symbols are absent, with
`crates/longhorn-agent-control/src/lib.rs` as positive control. Owns
`crates/longhorn-agent-tool-dispatch/**`, the control file, `Cargo.lock`.
Companions: `check:agent-tool-dispatch`. Admission: unmarked but builds; in
`qa`.

**`check:agent-tool-dispatch`** — role: aggregate (not in `qa`). Runs
`lint:agent-tool-dispatch`, `test:agent-tool-dispatch`,
`docs:agent-tool-dispatch`, `check:agent-tool-dispatch-release-absence`,
`check:api-reference`.

**`check:agent-control-skill`** — role: drift lock + fixtures. Runs
`scripts/verify-agent-control-skill.ts`. Owns
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
The `longhorn-docs` group contains exactly `qa:docs:links`,
`qa:docs:agent-defaults`, `qa:docs:paths`, and `qa:docs:catalog-links`.
It does not include `held-surface` or `host-protocol`; the `qa:docs`
aggregate still runs all six members.

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

`proof:guides-card126` runs `scripts/verify-guides-card126.ts` as a standalone
selector and remains one of the fourteen `proof:artifacts` members. The
`longhorn-getting-started-docs` group combines it with the two relevant link
selectors; its task-table token also routes an edit to that selector entry.
`longhorn-bridge-topology-conformance`, `longhorn-pack-typecheck`,
`longhorn-poodle-preview`, and `longhorn-documented-commands` each contain
their corresponding standalone selector. These groups do not replace the
artifact aggregate.

**`held-surface`** — role: docs proof. Runs `scripts/verify-held-surface.ts`.
Owns `docs/reference/held-surface.md`,
`docs/reference/api-surface.md`, `docs/guides/package-selection.md`.
Companions: `check:api-reference`. Limits: parses a fixed register table.

**`host-protocol`** — role: cross-language proof. Runs
`scripts/verify-host-protocol.ts`. Owns a recursive scan, not just `src`:
`crates/longhorn-tauri-*/**/*.rs` (the crate list comes from
`readdirSync(crates)` filtered to that prefix), `packages/*/**/*.ts` (the
whole package tree minus `node_modules`; the package list comes from
`readdirSync(packages)`), and the capability/permission examples under
`crates/longhorn-tauri-*/examples/{permissions,capabilities}/**` — permission
files are TOML with an `identifier =` field, capability files are JSON with a
`"permissions"` array. It asserts every invoke/event name has a counterpart
or a documented seam, every capability permission is declared, and every
allowed command exists. Propagation: a quoted `longhorn_*` command or
`longhorn://` event anywhere in a tauri crate or package, including tests and
examples, can fail this scan; a new `longhorn-tauri-*` crate or package
directory enters it automatically. Companions: `check:tauri-seam-strings`.
Admission: unmarked. Member of `qa:docs`. Limits: string inventory, not
types.

### Artifact proofs

`proof:artifacts` is `admission = "heavy"`, in `qa`, and runs fourteen
scripts in order. The artifact members with standalone selectors are
`proof:pack-typecheck`, `proof:guides-card126`,
`proof:bridge-topology-conformance`, `proof:poodle-preview`, and
`proof:documented-commands`; the guide selector also reuses one aggregate
member. The new standalone selectors do not change aggregate membership or
order. `proof:agent-tool-dispatch-source-consumer` is separate from this
aggregate. Other `verify-*.ts` names below are script members of
`proof:artifacts`, not dispatchable selectors. Members differ in what they do:
four are source-level checks that stage nothing, the rest pack TypeScript
and/or build an isolated Rust workspace.

**Shared inputs.** A change to any of these selects `proof:artifacts`:

- `scripts/proof-install.ts` — locked registry install into stages.
- `scripts/poodle-release.ts` — reads `bun.lock` and the installed
  `node_modules` Poodle copies.
- `scripts/longhorn-version.ts` — reads root `Cargo.toml`
  `workspace.package.version`.
- `scripts/msrv.ts` — reads `release-baselines/rust-toolchains.env` and root
  `Cargo.toml` `rust-version`; every member that builds a Rust stage uses it.
- `scripts/workspace-dependencies.ts` — reads root `Cargo.toml`
  `[workspace.dependencies]` and emits it into the generated proof manifests.
- `scripts/consumer-absence.ts`, `scripts/test-count.ts` — app-shell and
  history-system only.
- `scripts/*-proof/**` and `scripts/*-artifact-proof/**` implementation
  modules (artifacts, consumers, shared, types).
- Root `Cargo.lock` — copied into a disposable workspace by
  operation-notification, history-system, history-tree, native-content and
  greenfield. The bridge, command and settings split proofs do not read it.
- Root `package.json` + `bun.lock` — registry install and Poodle pins.
- Root `Cargo.toml` — workspace members and `[workspace.dependencies]`, read
  by several members; native-content also asserts `prototypes/native-content`
  is not a production member.

**Source-level members** (no pack, no stage, seconds):

| Member script | Helper modules | Inputs beyond shared | Standalone route | Bindings domain | Role and limits |
| --- | --- | --- | --- |
| `verify-bridge-topology-conformance.ts` | `examples/bridge-topology-proof/{common,proof,shape traces}.ts` | five shape files plus `proof.test.ts`, `declarations.json`, `README.md`; bridge crate manifest/source and query-only capability; `packages/longhorn/src/bridge/**`; root `node_modules` resolution | New: `proof:bridge-topology-conformance`, `longhorn-bridge-topology-conformance` | — | Runs the in-repo proof module and scans declared sources; proves import graphs and absent production edges |
| `verify-poodle-preview.ts` | `scripts/poodle-release.ts` | root `package.json`, `bun.lock`, installed `node_modules/@inflatable-cookie/poodle-{core,svelte}/package.json` | New: `proof:poodle-preview`, `longhorn-poodle-preview` | — | Checks pinned release versions, lock integrity entries, and the installed Svelte peer range |
| `verify-guides-card126.ts` | `scripts/generate-api-reference-card126.ts`, `scripts/longhorn-version.ts` | fixed docs list; local link targets; `Cargo.toml`; crate manifests/directories/README or library paths; package manifests/directories/README/export paths; `docs/reference/api-surface.md` | Existing: `proof:guides-card126`, `longhorn-getting-started-docs` | — | Checks guide content, local links, generated API inventory, and crate/package inventory |
| `verify-documented-commands.ts` | none | `effigy.toml` task names; every Markdown file under `examples/` except `node_modules`, `target`, and `gen` | New: `proof:documented-commands`, `longhorn-documented-commands` | — | Every `effigy <task>` named in an example Markdown file resolves |

**Staging members** (pack TypeScript and/or build an isolated Cargo
workspace, minutes each):

| Member script | Helper modules | Rust crates staged | Bindings domain | TypeScript packs | Inputs beyond shared | Root lock copied | Standalone route |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `verify-app-shell-proof.ts` | `proof-install.ts`, `consumer-absence.ts`, `poodle-release.ts`, `test-count.ts`, `longhorn-version.ts` | none | — | longhorn, longhorn-poodle-svelte | `examples/app-shell-proof/{split-shell,nucleus,loophole,common}/**` | no | None; the consumer staging and installed-package closure is not separately mapped |
| `verify-bridge-topology-artifacts.ts` | `scripts/bridge-topology-artifact-proof/**`, `msrv.ts`, `workspace-dependencies.ts`, `longhorn-version.ts` | core, bridge, tauri-bridge | `bridge` | longhorn, longhorn-tauri | `examples/bridge-topology-proof/**` | no | None; isolated Cargo packaging/consumer closure is not mapped to a member group |
| `verify-settings-composition-proof.ts` | `scripts/settings-composition-proof/**`, `msrv.ts`, `workspace-dependencies.ts`, `poodle-release.ts`, `longhorn-version.ts` | core, config, settings, settings-config, tauri-settings, tauri-config | — | longhorn, longhorn-poodle-svelte | `examples/settings-composition-proof/**`; `fixtures/{config,settings}/protocol-v1.json` | no | None; isolated Cargo and staged registry-install closure is not mapped to a member group |
| `verify-command-system-artifacts.ts` | `scripts/command-system-artifact-proof/**`, `msrv.ts`, `workspace-dependencies.ts`, `poodle-release.ts`, `longhorn-version.ts`, `consumer-absence.ts` | core, config, settings, command, command-config, command-settings, tauri-command | `commands` | longhorn, longhorn-poodle-svelte | `examples/command-system-proof/**` | no | None; isolated Cargo and staged registry-install closure is not mapped to a member group |
| `verify-history-system-artifacts.ts` | `proof-install.ts`, `consumer-absence.ts`, `workspace-dependencies.ts`, `test-count.ts`, `poodle-release.ts`, `msrv.ts`, `longhorn-version.ts` | core, history, tauri-history | `history` | longhorn, longhorn-poodle-svelte, longhorn-tauri | `examples/history-system-proof/**` | yes | None; copied lock, Rust package and staged TypeScript closure is not mapped to a member group |
| `verify-history-tree-artifacts.ts` | `proof-install.ts`, `poodle-release.ts`, `workspace-dependencies.ts`, `msrv.ts`, `longhorn-version.ts` | core, history, history-tree, tauri-history-tree | `history-tree` | longhorn, longhorn-poodle-svelte, longhorn-tauri | `examples/history-tree-artifact-proof/**` | yes | None; copied lock and isolated Rust/TypeScript consumer closure is not mapped to a member group |
| `verify-operation-notification-artifacts.ts` | `scripts/operation-notification-artifact-proof/**`, `msrv.ts`, `workspace-dependencies.ts`, `poodle-release.ts`, `longhorn-version.ts` | core, bridge, operation, notifications, tauri-operation, tauri-notifications | `operation`, `notifications` | longhorn, longhorn-poodle-svelte, longhorn-tauri | `examples/operation-notification-proof/**` | yes | None; copied lock and isolated Cargo/TypeScript closure is not mapped to a member group |
| `verify-native-content-artifacts.ts` | `proof-install.ts`, `poodle-release.ts`, `workspace-dependencies.ts`, `msrv.ts`, `longhorn-version.ts` | core, native-content, tauri-native-content-child-view, native-content-isolated-window, native-content-backing-surface | `native-content` | longhorn, longhorn-poodle-svelte | `examples/native-content-system-proof/**`; `examples/tauri-native-content-{backing-surface,child-view,isolated-window}-proof/**`; `prototypes/native-content/**`; three prototype Cargo manifests; fixture and generated protocol | yes | None; also invokes `check:bindings`, whose transitive compile closure is an existing planner gap |
| `verify-greenfield-card125.ts` | `proof-install.ts`, `msrv.ts`, `poodle-release.ts`, `longhorn-version.ts` | 24 crates (below) | — | longhorn, longhorn-poodle-svelte, longhorn-tauri | `examples/greenfield-compositions/**`; `fixtures/greenfield/card125/composition-matrix-v1.json` (read and checked by default; written only with `WRITE_GREENFIELD_RECEIPT=1`) | yes | None; generated Cargo workspace/24-crate compile closure is not mapped to a member group |
| `verify-pack-typecheck.ts` | `proof-install.ts`, `poodle-release.ts` | none | — | `@inflatable-cookie/longhorn`, `@inflatable-cookie/longhorn-poodle-svelte` | both `packages/<package>/**` trees; root `package.json`; `bun.lock`; registry Poodle at newest and floor; two peer-range stages | no | Existing selector; new `longhorn-pack-typecheck` group |

The 24 Rust crates greenfield stages are `longhorn-bridge`,
`longhorn-command`, `longhorn-command-config`, `longhorn-command-settings`,
`longhorn-config`, `longhorn-core`, `longhorn-display`, `longhorn-history`,
`longhorn-settings`, `longhorn-settings-config`, `longhorn-surface-transfer`,
`longhorn-surface-windowing`, `longhorn-surfaces`, `longhorn-surfaces-config`,
`longhorn-tauri-bridge`, `longhorn-tauri-command`, `longhorn-tauri-config`,
`longhorn-tauri-history`, `longhorn-tauri-settings`, `longhorn-tauri-transfer`,
`longhorn-tauri-windowing`, `longhorn-transfer`, `longhorn-windowing`,
`longhorn-windowing-config`.

**Companions and limits.** A source change that a member stages also selects
the ordinary gates for that source: `check:ts`/`test:ts`/`test:vitest` for
TypeScript, the Rust lane for crates, `check:bindings` for its domain. A
staged crate that a bindings domain renders also selects `check:bindings` at
the source, independent of the proof. The greenfield member reads and checks
`fixtures/greenfield/card125/composition-matrix-v1.json` in its default mode
(`verifyReceipt`: schema, inventories, graph/hierarchy fields and audits);
`WRITE_GREENFIELD_RECEIPT=1` switches it to the writer. A receipt-only edit
therefore selects `proof:artifacts`. The aggregate remains one heavy step;
member-level groups are separate routes and do not change its admission or
member order. Their input maps are limited to the declared standalone members;
the other staging members remain planner-owned.

## Cross-cutting input classes

### Rust crate dependents

`cargo test/clippy --workspace` is all-or-nothing. Only
`longhorn-agent-tool-dispatch` has scoped selectors. Everything else needs the
whole Rust lane: `lint:rust`, `lint:rust:features`, `test:rust`. A shared
crate (`longhorn-core` feeds 40+ manifests) and a leaf crate
(`longhorn-credential-keyring` has no in-tree dependents) select the same
three. `check:prototypes` additionally applies when a prototype
path-depends on the changed crate, and `proof:artifacts` when a proof member
stages the crate (the greenfield member alone stages 24). Evidence missing
for narrowing: no per-crate `test:rust:<crate>` selector exists, and `cargo`
alone is not an admissible Effigy selector.

### TypeScript package dependents

`check:ts` still loops all packages for the workspace aggregate. Package
selectors typecheck one package; `test:ts:<package>` runs its matching Bun or
Vitest tests. The `longhorn-ts` group also checks and tests both consumers
because the adapter and `longhorn-tauri` resolve `packages/longhorn` to
workspace source. `longhorn-tauri-ts` and `longhorn-poodle-svelte-ts` cover
their own packages. `check:svelte` applies only to a change that reaches
`packages/longhorn-poodle-svelte` source or its imports. Tests split:
`packages/longhorn` and `packages/longhorn-tauri` tests are `test:ts`;
`packages/longhorn-poodle-svelte` tests are `test:vitest`. `check:packages`
applies to any package manifest or source change. A package a proof member
packs also selects `proof:artifacts`: `longhorn` and `longhorn-poodle-svelte`
by most members, `longhorn-tauri` by bridge-topology-artifacts,
operation-notification, history-system, history-tree and greenfield. The
package groups do not cover package assembly or artifact proofs.

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
  `longhorn-docs` for the four bounded link, index-path, and agent-default
  checks. `qa:docs` remains the six-member docs aggregate when
  `held-surface` and `host-protocol` obligations apply or its owning validator
  is required.
- `docs/reference/{held-surface,api-surface}.md` or `docs/guides/package-selection.md`:
  add `held-surface`; add `proof:artifacts` when `api-surface.md` moved (its
  `scripts/verify-guides-card126.ts` member reads it).
- Any `docs/guides/*.md`, `docs/reference/README.md`, `docs/README.md`,
  `README.md`, or `examples/greenfield-compositions/README.md`: add
  `proof:artifacts` (member `scripts/verify-guides-card126.ts`). For
  `docs/guides/getting-started.md`, `longhorn-getting-started-docs` runs that
  standalone proof with the two link checks.
- `examples/**/*.md`: add `proof:artifacts` (member
  `scripts/verify-documented-commands.ts`); `longhorn-documented-commands`
  runs that standalone proof for a mapped Markdown path.
- Knowledge files: `qa:docs:paths` covers the index/contract set only.

No Rust or TypeScript suite is required for a docs-only change.

### Verifiers, proofs, fixtures and examples

`scripts/verify-*.ts` and `scripts/verify-source-consumer.sh` are not one
class. `proof:artifacts` owns only its fourteen members plus their helper
closures; other verifier implementations belong to the selector that runs
them. Route by selector, not by filename prefix, and treat each verifier
implementation as an owned input of its selector.

| Verifier implementation | Owning selector |
| --- | --- |
| `scripts/verify-app-shell-proof.ts`, `verify-bridge-topology-conformance.ts`, `verify-bridge-topology-artifacts.ts`, `verify-settings-composition-proof.ts`, `verify-command-system-artifacts.ts`, `verify-history-system-artifacts.ts`, `verify-history-tree-artifacts.ts`, `verify-operation-notification-artifacts.ts`, `verify-native-content-artifacts.ts`, `verify-poodle-preview.ts`, `verify-greenfield-card125.ts`, `verify-guides-card126.ts`, `verify-documented-commands.ts` | `proof:artifacts` |
| `scripts/verify-bridge-topology-conformance.ts` | `proof:bridge-topology-conformance` (also a `proof:artifacts` member) |
| `scripts/verify-poodle-preview.ts` | `proof:poodle-preview` (also a `proof:artifacts` member) |
| `scripts/verify-documented-commands.ts` | `proof:documented-commands` (also a `proof:artifacts` member) |
| `scripts/verify-guides-card126.ts` | `proof:guides-card126` (also a `proof:artifacts` member) |
| `scripts/verify-pack-typecheck.ts` | `proof:pack-typecheck` (also a `proof:artifacts` member) |
| `scripts/bridge-topology-artifact-proof/**`, `scripts/command-system-artifact-proof/**`, `scripts/operation-notification-artifact-proof/**`, `scripts/settings-composition-proof/**` | helper closures of their `proof:artifacts` members |
| `scripts/{proof-install,poodle-release,longhorn-version,msrv,workspace-dependencies,consumer-absence,test-count}.ts` | shared `proof:artifacts` helpers |
| `scripts/verify-held-surface.ts` | `held-surface` |
| `scripts/verify-host-protocol.ts` | `host-protocol` |
| `scripts/verify-tauri-seam-strings.ts` | `check:tauri-seam-strings` |
| `scripts/verify-consumer-isolation.ts` | `check:consumer-isolation` |
| `scripts/verify-repo-containment.ts` | `check:repo-containment` |
| `scripts/verify-agent-control-release-absence.ts` | `check:agent-control-release-absence` |
| `scripts/verify-agent-tool-dispatch-release-absence.ts` | `check:agent-tool-dispatch-release-absence` |
| `scripts/verify-agent-tool-dispatch-source-consumer.ts` | `proof:agent-tool-dispatch-source-consumer` |
| `scripts/verify-agent-control-skill.ts` | `check:agent-control-skill` |
| `scripts/verify-source-consumer.sh` | `release:source-consumer` |
| `scripts/verify-private-candidate-docs-card127.ts` | `release:gates` (private-candidate) |
| `scripts/verify-private-candidate-docs-card127.test.ts` | `test:release-tooling` |
| any other `scripts/verify-*.ts` | unresolved until its owning selector is established |

`examples/**` is not one rule either: `examples/*/src-tauri/**` and
`examples/*/rust/**` are root workspace members, so they select the Rust lane;
an example root a proof member stages (see the staging table) selects
`proof:artifacts`; the containment scans walk the whole tree.

Fixtures do not have one owner. Route by reader:

| Fixture | Readers |
| --- | --- |
| `fixtures/bridge/protocol-v1.json` | `check:bindings`, `test:ts` |
| `fixtures/commands/protocol-v1.json` | `check:bindings`, `test:ts`, `test:rust` |
| `fixtures/config/protocol-v1.json` | `check:bindings`, `test:ts`, `test:vitest`, `test:rust`, `proof:artifacts` (settings member) |
| `fixtures/greenfield/card125/composition-matrix-v1.json` | `proof:artifacts` (greenfield member) |
| `fixtures/history-tree/protocol-v1.json` | `check:bindings` |
| `fixtures/history/protocol-v1.json` | `check:bindings`, `test:ts`, `test:vitest`, `test:rust` |
| `fixtures/layout/protocol-v1.json` | `check:bindings`, `test:ts` |
| `fixtures/layout/surface-bound-conformance-v1.json` | `check:bindings`, `test:ts`, `test:vitest` |
| `fixtures/layout/window-bound-conformance-v1.json` | `check:bindings`, `test:ts`, `test:vitest` |
| `fixtures/layout/surface-bound-registered-authority-v1.json` | `test:rust` (longhorn-bindings unit test) |
| `fixtures/licence/protocol-v1.json` | `check:bindings`, `test:ts` |
| `fixtures/native-content/protocol-v1.json` | `check:bindings`, `test:ts`, `test:vitest`, `proof:artifacts` (native-content member) |
| `fixtures/notifications/protocol-v1.json` | `check:bindings`, `test:ts` |
| `fixtures/operation/protocol-v1.json` | `check:bindings`, `test:ts`, `test:rust` |
| `fixtures/parity/projection-v1.json` | `test:rust`, `test:vitest` |
| `fixtures/release/card127/private-0-1-candidate-v1.json` | `release:gates` (private-candidate) |
| `fixtures/settings/protocol-v1.json` | `check:bindings`, `test:ts`, `test:vitest`, `test:rust`, `proof:artifacts` (settings member) |
| `fixtures/surface-transfer/protocol-v1.json` | `check:bindings`, `test:ts`, `test:vitest` |
| `fixtures/surfaces/protocol-v1.json` | `check:bindings`, `test:ts`, `test:vitest` |
| `fixtures/surfaces/surface-bound-registered-authority-v1.json` | `test:rust` (longhorn-bindings unit test) |
| `fixtures/transfer/protocol-v1.json` | `check:bindings`, `test:ts`, `test:vitest` |
| `fixtures/update/protocol-v1.json` | `check:bindings`, `test:ts` |
| `fixtures/agent-tool-dispatch-provider-free/main.rs` | `proof:agent-tool-dispatch-source-consumer` |
| any other `fixtures/**` file | unresolved unless a selector entry or a row above names a reader |

Every file under `fixtures/`, `scripts/`, `examples/`, `crates/`, `packages/`
and `prototypes/` is also scanned by `check:consumer-isolation` and
`check:repo-containment`, which select whole trees rather than files.

### Added, deleted, renamed files

- New crate: the new directory is already read before registration by
  `check:consumer-isolation` and `check:repo-containment` (recursive scans),
  `check:api-reference` (counts every `crates/*` directory against `cargo
  metadata` and fails on a mismatch) and `proof:artifacts` (the guides member
  compares the same inventory); registering it in the root workspace `members`
  adds the full Rust lane and `fmt:rust` (member list). Compile ownership is
  unresolved until registration.
- New package: directory discovery is by `packages/*`, so the recursive scans
  and inventory checks read it before registration; registration (the root
  `workspaces` glob already covers `packages/*`) adds `check:ts`,
  `check:packages`, `test:ts`/`test:vitest`, and `proof:pack-typecheck` when
  it is a pack target.
- New `longhorn-bindings` domain: `check:bindings` will not see it until the
  task loop in `effigy.toml` names it -- a real coverage gap.
- New `longhorn-tauri-*` crate: `check:tauri-seam-strings` and `host-protocol`
  read `crates/longhorn-tauri-*`, so they pick it up automatically.
- Rename or move a file: select the union of the old/deleted path and the
  new/added path. Gates with fixed output paths need the old path to see the
  deletion -- moving `packages/longhorn/src/history/generated/protocol.ts` out
  of `generated/` still selects `check:bindings` because the expected artifact
  is now missing. Registration lists (workspace `members`, the
  `packages/*/tsconfig.json` loop, the hardcoded bindings domain loop,
  `check:svelte`'s hardcoded tsconfig) do not follow a rename automatically;
  treat a new path no selector reaches as unresolved.
- Delete: select by the deleted path's role. `check:repo-containment` does not
  check existence, only that a named path stays inside the tree, so a deleted
  in-tree Cargo path dependency passes it and fails later at `cargo metadata`
  -- the Rust lane, `check:bindings`, or a proof that packages the crate. A
  deleted package export fails `check:ts`/`check:packages`; a deleted proof
  fixture fails `proof:artifacts`; a deleted file no selector reads selects
  nothing until a gate fails. Treat deletion of a fixed-path input as
  unresolved when no selector names it.

### Staged, unstaged, relevant untracked inputs

Every gate reads the working tree, not the Git index: `--locked` compares
manifests to `Cargo.lock`/`bun.lock`, and the scan selectors walk the files.
Staged vs unstaged does not change the answer. Untracked files under a scanned
tree are seen (the scan selectors walk directories; the graph's
`git status --porcelain` reports `??`). An unregistered crate or package is
still read by the recursive scanners and inventory checks:
`check:consumer-isolation` and `check:repo-containment` walk the new
directory, `check:api-reference` counts `crates/*` directories against `cargo
metadata` and fails on a mismatch, package manifests are discovered by a
`packages/*` directory glob, and `proof:artifacts` (the guides member)
compares the same inventories. Compile ownership stays unresolved until the
crate or package is registered; do not treat the directory as "no checks".
Ignored paths (`target/`, `node_modules/`, `.effigy/`, `.svelte-kit/`) are
never selection inputs.

## Opaque and global inputs

| Input | Safe selection | Why |
| --- | --- | --- |
| `Cargo.lock` | conservative: full Rust lane + `check:bindings` + `check:api-reference` + `check:agent-control-release-absence` + `check:agent-tool-dispatch-release-absence` + `proof:artifacts` + `check:prototypes` + `release:floor`, plus the release-only `docs:rust` and `advisories` gates | no selector proves the lock alone; every `--locked` reader depends on it -- the Rust lane, `check:bindings` (generator compile), `check:api-reference` (`cargo metadata --locked`), `docs:rust` (`cargo doc --locked`, release-only), `advisories` (`cargo deny` reads the resolved graph, release-only), the absence proofs, and five proof members that copy it into disposable workspaces. Narrower selection is unresolved |
| `bun.lock` | conservative: `bootstrap:deps`, full TypeScript lane, `proof:artifacts`; bounded: `longhorn-pack-typecheck`, `longhorn-poodle-preview` | proofs install from the lock; the standalone Poodle and pack proofs check the lock against installed Poodle metadata and staged registry resolution |
| `package.json` (root) | `bootstrap:deps`, `check:ts`, `check:svelte`, `proof:artifacts`; bounded: `longhorn-bridge-topology-conformance`, `longhorn-pack-typecheck`, `longhorn-poodle-preview` | dev pins and peer ranges |
| `effigy.toml` (global or unclassified changes) | `needs_planner`; no automatic board fallback | planner-owned conservative selection includes the board and the existing `check:release-gates`, `check:runner-tools`, `test:release-tooling`, and `proof:artifacts` obligations; task-table-only edits may route only through the exact selector tokens above |
| `config/release.toml` | `check:release-gates`, `test:release-tooling`, private-candidate proof, `check:runner-tools` | release-gate alignment and runner-tool mapping |
| `rust-toolchain.toml` | conservative: `fmt:rust` (rustfmt component), `lint:rust`, `lint:rust:features`, `test:rust`, `docs:rust` (release), `check:bindings`, `check:api-reference`, `check:agent-control-release-absence`, `check:agent-tool-dispatch-release-absence`, `proof:artifacts` (members invoking unqualified `cargo`), `check:prototypes` (release), `release:source-consumer` (release), `ci:rehearse` (release) | rustup resolves the pinned channel and components for every unqualified `cargo`/`rustfmt` invocation, so a channel or component change can break any of these. Unlike the MSRV file, no gate asserts the channel itself. `release:floor` uses `rustup run <msrv>`, not this channel. Narrower selection is unresolved |
| `release-baselines/rust-toolchains.env` | `release:floor`, `proof:artifacts`, `release:source-consumer` | MSRV gates, `msrv.ts` manifest generation, and the source-consumer `rust-version` (`scripts/verify-source-consumer.sh` sources the file) |
| `.github/workflows/**` | `check:runner-tools`, `ci:rehearse` | install-step mapping and clean-runner rehearsal |
| generated `.ts` under `packages/*/src/**/generated/**` | `check:bindings`, `check:ts` | drift vs compile |
| `crates/*/bindings/**` | unresolved | ts-rs `#[ts(export)]` per-type output; no selector reads or diffs it (see Coverage gaps) |
| `target/**`, `node_modules/**`, `.effigy/**` | none | ignored build/cache state |

Anything not in this table and not matched by a selector above is unresolved:
declare it broad and let the planner judge, never treat it as no checks.

## Representative synthetic change sets

| # | Change | Expected selectors | Reason |
| --- | --- | --- | --- |
| 1 | `docs/guides/getting-started.md` prose edit | `qa:docs`, `longhorn-getting-started-docs` | catalogue/link checks plus the standalone guide proof; `proof:artifacts` keeps the same aggregate membership |
| 2 | leaf Rust crate: `crates/longhorn-credential-keyring/src/**` | `fmt:rust`, `lint:rust`, `lint:rust:features`, `test:rust`, `check:consumer-isolation`, `check:repo-containment` | no per-crate selector and no proof stages it, but the lane is workspace-wide; `examples/update-licence-proof/rust/harness` depends on it, and the lane covers that dependent. A crate a proof stages (for example `longhorn-history`) also selects `proof:artifacts` |
| 3 | shared Rust type: `crates/longhorn-history/src/**` | Rust lane + `check:bindings` + `check:ts` + `test:ts` + `test:vitest` + `proof:artifacts` + `check:prototypes` | history is a bindings domain; `longhorn-history` feeds `longhorn-bindings`, `history-tree`, `tauri-history`, `prototypes/history-tree` |
| 3b | transitive prototype input: `crates/longhorn-display/src/**` or `crates/longhorn-surfaces-config/src/**` | Rust lane + `check:prototypes` (release gate) + `proof:artifacts` | reached by the prototypes only through `longhorn-windowing`/`longhorn-gpui-windowing` (display) and `longhorn-transfer` (surfaces-config); the greenfield member stages both |
| 3c | transitive prototype-only input: `crates/longhorn-url/src/**` | Rust lane + `check:prototypes` (release gate) | reached only through `longhorn-licence`/`longhorn-update`; no artifact member stages `longhorn-url` |
| 3d | behavioural generator input: `crates/longhorn-licence/src/key.rs` parser or `key_conformance_cases()` change | Rust lane + `check:bindings` + `test:ts` | the generator emits `packages/longhorn/src/licence/generated/key-conformance.json` from `key_conformance_cases()`, and `packages/longhorn/tests/licence/key.test.ts` reads that JSON through `packages/longhorn/src/licence/key.ts`; no public type moves, so a type-only rule would miss the committed-bytes drift |
| 4 | leaf TS package: `packages/longhorn-tauri/src/transport/**` | `check:ts`, `test:ts`, `check:packages`, `check:tauri-seam-strings`, `host-protocol`, `proof:artifacts` | no package dependents, but five proof members pack `longhorn-tauri`; typecheck + its tests + seam/protocol scans + the packed-artifact aggregate |
| 5 | proof generator/fixture: `scripts/verify-history-tree-artifacts.ts`, `scripts/workspace-dependencies.ts`, `fixtures/native-content/protocol-v1.json`, or `fixtures/greenfield/card125/composition-matrix-v1.json` | `proof:artifacts`, `check:consumer-isolation`, `check:repo-containment`, plus `test:ts`/`test:vitest` if a Bun test reads the fixture, plus `check:bindings` when it is also a golden `fixtures/<domain>/protocol-v1.json` | implementation modules and consumed fixtures are proof inputs; the greenfield receipt is read and checked by default |
| 5b | binding conformance fixture: `fixtures/layout/surface-bound-conformance-v1.json` or `fixtures/layout/window-bound-conformance-v1.json` | `check:bindings`, `test:ts`, `test:vitest`, `check:consumer-isolation`, `check:repo-containment` | generated conformance outputs, byte-checked by the layout domain and imported by the Bun and Svelte layout suites; no `proof:artifacts` member reads them |
| 6 | opaque: `Cargo.lock` | conservative: Rust lane + `check:bindings` + both `check:agent-*-release-absence` + `proof:artifacts` + `check:prototypes` + `release:floor` | no lock-only selector; build, generator and copied-workspace consumers all depend on it |
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
- Package TypeScript selectors typecheck one package and select its Bun or
  Vitest tests. `check:ts`, `test:ts`, and `test:vitest` remain aggregate
  selectors; consumer propagation from `longhorn` is declared by `longhorn-ts`.
- `check:bindings`'s domain list is hardcoded in `effigy.toml`. A new
  generator domain is uncovered until the loop is edited. No selector detects
  that.
- `check:svelte` hardcodes `packages/longhorn-poodle-svelte/tsconfig.json`; a
  second Svelte package would be uncovered.
- `crates/*/bindings/**` is committed ts-rs `#[ts(export)]` output under
  `longhorn-core`, `longhorn-licence` and `longhorn-update`. No selector reads
  or diffs it: `check:bindings` compares only each domain's explicit
  `packages/longhorn/src/**` and `fixtures/**` artifacts. The only owner is
  the `bindings` feature compile; whether it should be diff-checked or deleted
  is unresolved.
- Generated and ignored paths are invisible to Effigy's graph, so `graph
  affected` cannot prove `packages/*/src/**/generated/**`,
  `crates/*/bindings/**`, or lockfiles are covered. The honest answer for
  those inputs is the declared selector above, or an explicit unresolved
  classification where none exists.
- `Cargo.lock` and `bun.lock` have no narrow selector. The conservative sets
  above are declared; the missing evidence is a lock-only gate with a
  reproducible output digest.
- Renames and deletions: no gate consumes an old-path/new-path pair, and no
  scan tests existence. A moved file is selected by the union of both paths; a
  rename or delete that removes a fixed-path input (generated artifact,
  workspace member, package entry, packed crate) has no selector that reports
  the removal until `check:bindings`, `check:ts`, `check:packages` or the Rust
  lane fails on metadata. Cases no selector names are unresolved.
- No gate asserts the `rust-toolchain.toml` channel or component set itself;
  its consumers are declared in the opaque-input table. `effigy.toml` task
  definitions and `.github/workflows/**` stay conservative beyond
  `check:runner-tools`.
- `proof:artifacts` remains one heavy aggregate with unchanged membership and
  order. Maintained groups can invoke only the five standalone proof members
  listed in the artifact section; the remaining members stay planner-routed.
- Prototype locks (`prototypes/*/Cargo.lock`) and the root lock are separate;
  `sync:prototype-locks` is the only writer and is not a gate.
- Untracked files outside a manifest/glob reach select nothing. That is
  unresolved, not "no checks".
- Verifier and fixture routing is reader-based, not name-based. A
  `scripts/verify-*.ts` with no selector wiring, or a `fixtures/**` file no
  reader names, is unresolved -- it is not routed to `proof:artifacts` by its
  filename or its location.

## Known gaps and limits

Items the source could not settle, carried for Effigy's selection-contract
phase rather than guessed:

- `longhorn-core` is declared as a whole-crate generator input, and the
  fifteen domain crates as whole-crate inputs. The complete per-domain
  behavioural call set is not enumerated; only `store_compatibility.rs`'s
  `ts(export)` attribute, the cross-domain ids/constants, and
  `longhorn_licence::key_conformance_cases()` are established from source.
- The agent-control absence proof is declared as a crate-wide source input for
  `crates/longhorn-agent-control/**`. The exact feature-gated files and the
  full rlib marker set beyond `src/server/mcp.rs` and the script's own marker
  list are not enumerated.
- Dynamic proof outcomes, exact per-selector costs, and the registry, advisory
  database and bun-link machine state are not measured here; cost notes come
  from comments already kept next to those tasks.
- Effigy's `graph affected` output cannot establish coverage for generated,
  ignored or lock inputs (see Coverage gaps).
- Any `fixtures/**` file or `scripts/verify-*.ts` with no reader named in this
  map stays unresolved; the map does not invent one.
- `check:bindings` and every proof that invokes the bindings generator: the
  generator's transitive compile dependencies are not enumerated here. That
  includes crates the artifact proofs compile but don't stage, such as
  `longhorn-url` and the `longhorn-surfaces-config` examples. A change to any
  crate the generator compiles is unresolved for narrowing, so select
  `check:bindings` and the invoking proofs conservatively (review round 6,
  PR #61). The `longhorn-bindings` group declares the closure as per-crate
  `cargo-package:` and `path:` gap entries, so ordinary tokens for those
  crates plan `needs_planner`; what the generator reads per crate stays
  unmapped.
- `check:agent-control-release-absence`: its local Rust dependency closure
  (`longhorn-core`, `longhorn-config`, `longhorn-tauri-config`) and its
  committed JS build input, the Tauri shim asset, are compile inputs. They
  are distinct from the marker sources the byte scan looks for. A change to
  any of them selects the gate; exact marker-source coverage remains
  unresolved. The `longhorn/agent-control-absence` group declares the five
  compile crates (`longhorn-agent-control` and `longhorn-tauri-agent-control`
  included) as `cargo-package:` and `path:` gap entries plus the opaque
  `input:agent-control-marker-source-coverage` token, so ordinary tokens for
  them plan `needs_planner` (review round 6, PR #61).

## Evidence and limits

Read to build this map: `effigy.toml`, `config/release.toml`,
`scripts/README.md`, the `scripts/verify-*` scripts, every
`scripts/*-proof/**` and `scripts/*-artifact-proof/**` implementation module,
the shared `scripts/{proof-install,poodle-release,longhorn-version,msrv,
workspace-dependencies,consumer-absence,test-count}.ts` helpers,
`crates/longhorn-bindings/{Cargo.toml,src}` including `generation.rs`
`check_artifacts` and its per-domain `Artifact` paths,
`packages/longhorn/src/agent-control/{inject,shim}.ts` and
`scripts/agent-control-shim.ts`, each `packages/*/package.json` and
`tsconfig.json`/`vitest.config.ts`, the root `Cargo.toml`, `deny.toml`,
`.github/workflows/{ci,release}.yml`,
`docs/knowledge/contracts/release.md`, Effigy guides 076 (code graph and
agent workflows), 080 (host-wide validation admission), and 081 (bounded QA
groups), plus Effigy contract 051.

This map records what the commands read and what they do not. It was not built
by running the full board or measuring stable timings; existing cost figures
come from the comments Longhorn already keeps next to those tasks. Five
artifact members dispatch through standalone selectors with bounded groups;
the remaining artifact members still run only through `proof:artifacts`.
`proof:agent-tool-dispatch-source-consumer` is standalone and is not a member
of that aggregate.
Independent review should check each entry against the manifest or script it
names and walk the eleven synthetic change sets on paper.
