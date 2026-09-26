# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

### [ ] `set_file_input` size cap is checked only at the MCP edge — 2026-09-26
- Friction: g02.048 review found `SetFileInputRequest::validate` (8 MiB cap) runs in `SetFileInputArgs::into_request`, not in `TauriControlHandler::set_file_input`; its doc comment claims both. The shim re-checks names, base64, and empty lists, but not size.
- Impact: none over MCP or the stdio carrier; a direct in-process `ControlHandler` caller skips the cap.
- Plausible fix: call `request.validate()` in the Tauri handler, or correct the doc comment.
- Surface: `crates/longhorn-agent-control/src/tools.rs`; `crates/longhorn-tauri-agent-control/src/handler.rs`.

### [ ] The Tauri shim copy has no drift check — 2026-09-26
- Friction: `crates/longhorn-tauri-agent-control/src/agent_control_shim.js` is a hand-synced copy of `packages/longhorn/src/agent-control/shim.ts`. g02.048's reviewer compared the new functions by hand.
- Impact: a TS shim fix can pass `bun test` and never reach the webview.
- Plausible fix: generate the JS from the TS source, or add a `qa` check that fails on divergence.
- Surface: both shim files; `effigy.toml` `qa`.

### [ ] `release:bump` can under-report or partly apply — 2026-09-26
- Friction: g02.049 review found `bumpRelease().changed` omits lock rewrites, `bumpPackageManifest` silently skips a manifest whose `"version"` line changes shape, and a failure after the tracked-file writes leaves a partly bumped tree.
- Impact: a bump can report idempotent while a lock moved, or miss a package after a reformat; recovery is a manual revert.
- Plausible fix: include lock paths in `changed`; make the manifest bump throw on no match; stage writes and apply after lock sync and API regeneration succeed.
- Surface: `scripts/release-bump.ts`.

### [ ] `release status --check-gates` fails a green, already-versioned candidate — 2026-09-26
- Friction: on the manually prepared `0.2.1` candidate, `effigy release status --check-gates` passed all seven gates, then exited nonzero because `[Unreleased]` is empty and it cannot propose a next version.
- Impact: a green gate run reads as a failure; the release owner must read the gate lines, not the exit code.
- Plausible fix: report gate results separately from next-version proposal, or accept an already-versioned candidate that matches the manifests.
- Surface: effigy-release `status --check-gates`; `CHANGELOG.md` `[Unreleased]`; manual candidate route (`g02.046`).
- Upstream: Effigy. Longhorn-owned `release:bump` and the runbook treat the gate lines as the evidence.

### [ ] `cargo update` of path crates re-resolves third-party prototype lock entries — 2026-09-25
- Friction: `cargo update --offline --precise 0.2.1 -p longhorn-*` in `prototypes/gpui-composition` rewrote Longhorn path versions and also moved registry lines (`windows-sys` 0.60.2 → 0.52.0/0.61.2 and several dependents). The 0.2.0 candidate lock diff was version-only.
- Impact: a naive Cargo refresh cannot be the excluded-lock sync; third-party movement is a g02.046 stop condition.
- Plausible fix: keep the surgical path-version rewrite in `sync:prototype-locks`, or teach cargo a no-re-resolve mode for path version bumps.
- Surface: `scripts/sync-prototype-locks.ts`; `prototypes/*/Cargo.lock`; `cargo update`.

### [ ] Release version bump stales the workspace-excluded prototype locks — 2026-09-22
- Friction: `effigy release prepare --version 0.2.0` bumps `workspace.package.version` and syncs the root `Cargo.lock`, but the eight `prototypes/*/Cargo.lock` files pin the `longhorn-*` crates at the old version. The `prototypes` gate then fails (`cargo check --locked` refuses a stale lock), and `[release] sync-files` supports only `Cargo.lock` and `package.json` (effigy-release `resolve_sync_files`), so the tool cannot fix it. Pre-bumping by hand also blocks the tool (`--version ... must be greater than current version`).
- Impact: a version-bumping release cannot use `effigy release prepare`; the 49 internal pins, the root lock, the eight prototype locks, and the changelog promotion had to be done by hand with the gates run manually.
- Plausible fix: teach release sync about workspace-excluded lockfiles, add a repo-owned pre-gate step that refreshes them, or make the prototype manifests version-agnostic. g02.046 added `effigy sync:prototype-locks` as that pre-gate step; g02.049's `release:bump` runs it. `release prepare` still cannot see the excluded locks.
- Surface: `config/release.toml` (`sync-files`); `prototypes/*/Cargo.lock`; `scripts/sync-prototype-locks.ts`; `scripts/release-bump.ts`; effigy-release `resolve_sync_files`; `Cargo.toml` internal version pins.
- Upstream: Effigy. No pre-gate hook for workspace-excluded locks.

### [ ] Consumer repoint dispatches need a declared cross-repo Rust identity edge — 2026-09-16
- Friction: soundcheck (g04.034) and soundcheck-library (g01.008) share one `longhorn-core`/`longhorn-history` identity: soundcheck reaches the library by path, so its lock carried both the new `?tag=v0.1.0` copy and the library's older `?rev=` copy. Dispatched in parallel with no `queue.dependsOn`, soundcheck hit `E0308` cross-identity errors and blocked until the library merged.
- Impact: a coupled pair ran concurrently and one lane stalled; the dependency should have been declared at submission, not discovered from the failure.
- Plausible fix: declare `queue.dependsOn` for consumer pairs that share a crate identity (soundcheck → soundcheck-library), or group them as one lane.
- Surface: Longhorn consumer dispatch (g02.014); soundcheck's path dep on soundcheck-library.

### [ ] Release QA uses the runner's latest stable clippy, local uses older — 2026-09-16
- Friction: `effigy qa` in `release.yml` runs on `dtolnay/rust-toolchain@stable`, which tracks the newest stable. Rust 1.98 added `clippy::chunks_exact_to_as_chunks`; the same gate was green locally on 1.97 and red on the runner, stopping the dry run before any publish step.
- Impact: a release can fail on a lint that did not exist when the code was written or last validated locally, with no local signal.
- Plausible fix: pin the runner's stable toolchain (or assert local/runner parity) so new lints are adopted deliberately rather than mid-release; locally, `rustup update` and re-run `effigy qa` before release.
- Surface: `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `release-baselines/rust-toolchains.env`.

### [ ] Broken intra-doc link escapes qa and only fails the release gate — 2026-09-15
- Friction: `longhorn-tauri-agent-control`'s crate-root doc comment linked `[`ControlHandler`]`, which is not re-exported at the root. `effigy qa` does not run rustdoc, so the crate was green locally while the release gate `rustdoc` (`RUSTDOCFLAGS="-D warnings"`, `rustdoc::broken-intra-doc-links`) failed on the first `release prepare --check-gates`.
- Impact: a release-gate-only failure that local qa never surfaces; the first real gate run stops before `qa`, `floor`, or `source` run at all.
- Plausible fix: add a bounded rustdoc check to `qa`, or keep the release-shaped placement and call it out in the release runbook.
- Surface: `crates/longhorn-tauri-agent-control/src/lib.rs`; `effigy.toml` `qa`; `config/release.toml` gate `rustdoc`.

### [ ] private-candidate release gate asserts exact CHANGELOG prose — 2026-09-15
- Friction: `verify-private-candidate-docs-card127.ts` requires literal substrings in `CHANGELOG.md` ("deterministic private `0.1.0` candidate", "36 Rust", "seven consumer"). A legitimate clarification that dropped one word and reflowed two lines failed the release gate's first step, before any real check ran.
- Impact: any human edit near the Card 127 note can block the entire release gate run for reasons unrelated to the release.
- Plausible fix: read the candidate version and counts from the frozen receipt and assert them structurally rather than matching prose.
- Surface: `scripts/verify-private-candidate-docs-card127.ts`; `config/release.toml` gate `private-candidate`.

### [ ] Poodle Text has no wrap control for long unbroken strings — 2026-09-15
- Friction: settings pages render digests, paths, and archive hashes that need `overflow-wrap: anywhere`; Poodle `Text`/`Code` expose no wrap control, so g02.020 dropped two local rules with no Poodle-owned replacement.
- Impact: long digests and paths in the Backup/Restore settings pages can overflow their Surface on narrow widths.
- Possible fix: a `wrap` prop on `Text` (and `Code`) mapping to `overflow-wrap`.
- Surface: `@inflatable-cookie/poodle-svelte` Text/Code; `longhorn-poodle-svelte` Backup/Restore settings pages.

### [ ] Rust closeout scans an intentionally non-workspace example — 2026-09-07
- Friction: Northstar Rust `closeout` runs Cargo metadata on `examples/greenfield-compositions/common-rust`, which intentionally has no workspace membership or local `[workspace]` table.
- Impact: required everyday-authoring closeout stops before collecting evidence for an unrelated, valid workspace crate.
- Plausible fix: honor profile ownership/exclusions before probing standalone Cargo manifests.
- Surface: Northstar Rust quality `closeout`, Longhorn greenfield example manifests.

### [ ] Cold code-graph refresh exceeds its default budget — 2026-09-07
- Friction: `effigy graph explore` timed out after 120 seconds while indexing 2,135 of 2,588 files in a fresh worktree.
- Impact: graph-first code navigation cannot answer the initial ownership query without a separate warm-up or timeout override.
- Plausible fix: make cold indexing resumable within the default budget or surface a bounded automatic continuation.
- Surface: `effigy graph explore`, cold worktree index.

### [ ] Isolated proof `bun install` can miss cache package dirs — 2026-09-05
- Friction: `effigy proof:artifacts` isolated consumers sometimes fail
  `bun install --ignore-scripts` with `FileNotFound: failed opening
  cache/package/version dir` for `vite` and friends after a successful
  resolve. Retrying the same proof often passes.
- Impact: `qa` / `proof:artifacts` is not deterministic on a warm Bun cache
  when many isolated installs run back to back.
- Plausible fix: give each isolated proof its own `BUN_INSTALL_CACHE_DIR`,
  or retry that install once on this class of cache miss.
- Surface: `scripts/*artifact*`, `scripts/settings-composition-proof/`,
  `effigy proof:artifacts`.

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

## 2026-09-07 — Finder metadata breaks host-protocol scan

`qa:docs` failed because `verify-host-protocol.ts` recursed into the ignored
`packages/.DS_Store` file as a directory. Preserving that file outside the
scan root restored PASS. Follow-up: filter directory entries before recursion.

## 2026-09-15 — npm registry serves metadata for a version whose tarball 404s

- Friction: `@types/node` 26.6.0 published minutes earlier; `npm view` and the
  packument listed it, but the CDN returned 404 for the tarball for ~2 minutes.
  The settings-composition proof's fresh `bun install` (unpinned resolution)
  failed on it; a polled retry went green with no code change.
- Impact: any proof or gate that resolves a fresh registry release at
  install time can fail spuriously right after an upstream publish, and the
  failure reads as a broken gate.
- Plausible fix: retry-with-backoff around registry installs in proof
  staging, or pin the transitive resolution in the staged manifests.
- Affected surface: `scripts/settings-composition-proof/`,
  `scripts/verify-pack-typecheck.ts`, any future registry-install proof.

## 2026-09-22 — Vitest SSR import test times out under a full `effigy qa` run

- Friction: `effigy qa` failed in `test:vitest` only on
  `packages/longhorn-poodle-svelte/tests/native-content/ssr.test.ts`
  ("imports without browser globals") with `Test timed out in 5000ms`, while
  the Rust workspace was compiling in the same run. An immediate
  `effigy test:vitest` re-run passed 36/36 files, 139/139 tests with no change.
- Impact: a green tree can read red once, and the failure text points at an
  unrelated native-content import rather than at load.
- Plausible fix: give that test a longer `testTimeout`, or warm the module
  graph before the timed section; alternatively keep the Rust and TS gates in
  separate invocations so transform/collect does not compete with a full
  workspace build.
- Affected surface: `packages/longhorn-poodle-svelte/tests/native-content/ssr.test.ts`,
  `effigy test:vitest`, `effigy qa`.
