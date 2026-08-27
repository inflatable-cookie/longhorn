# Papercuts

Small, actionable friction found during agent work. Agents append entries when
they hit a solvable hurdle; they do not stop the current task to fix one.

## Open

<!-- Keep entries short. Append newest entries at the top. Do not include secrets. -->

### [ ] Release gates execute in name order, so cheap-first is unbuyable — 2026-08-15
- Friction: `[release.gates]` is written cheapest-first, but effigy sorts by
  gate name. A measured run is advisories, floor, private-candidate,
  prototypes, rustdoc, source, workspace — the 145s MSRV floor runs before the
  38ms candidate check, so a release that will fail on the cheap gate pays for
  the expensive one first.
- Impact: ~2.5 wasted minutes per failing release run; the comment claiming
  "cheapest first" was false in two files before this was measured.
- Possible fix: an ordering key in effigy's `[release.gates]`, or execution in
  declaration order. Renaming gates to sort correctly is not available —
  `verify-private-candidate-docs-card127.ts` asserts two gate lines verbatim.
- Surface: `config/release.toml`, `effigy release gates`.

### [ ] Endpoint URL validation duplicated across capability crates — 2026-08-07
- Friction: `longhorn-update::EndpointUrl` and `longhorn-licence::ActivationUrl`
  independently parse and validate an HTTPS URL. The rules differ on purpose
  (update allows loopback HTTP for a local shim; activation does not, because
  its requests carry credentials), but the parsing is the same thirty lines
  twice.
- Impact: a parsing bug fixed in one is not fixed in the other. The IPv6
  bracket case was caught in `longhorn-update` by a test; nothing guarantees
  the licence side gets the same scrutiny.
- Possible fix: promote a shared URL primitive when a third caller appears.
  Not `longhorn-core` today — two callers do not justify growing core an
  HTTP concept, and coupling two optional capability crates so one cannot be
  composed without the other is worse than the duplication.
- Surface: `crates/longhorn-update/src/source.rs`,
  `crates/longhorn-licence/src/activation.rs`.

### [ ] `.agents.local.env` is a convention with no gitignore entry — 2026-08-19
- Friction: the worker-worktree fallback expects an ignored
  `.agents.local.env` carrying `AGENTS_WORKTREE_CONTAINER_DIR`, but the
  repo's `.gitignore` does not cover it, so creating the file makes the
  shared checkout dirty. Both worker dispatches so far routed the container
  question to the operator instead (`~/Dev/worktrees`).
- Impact: every dispatched worker without a launcher worktree must ask the
  operator the same question again.
- Plausible fix: add `.agents.local.env` to `.gitignore`, then create the
  file with `AGENTS_WORKTREE_CONTAINER_DIR=/Users/tom/Dev/worktrees`.
- Surface: `.gitignore`, orchestrator/worker handoff loop.

## Closed

### [x] Doctor schema rejects inline `{ rhai = ... }` task values — 2026-08-27
- Friction: `"agent-control:install-skill" = { rhai = "..." }` ran correctly
  but `effigy doctor` flagged `rhai` as an unsupported task-table key; the
  task was withdrawn post-merge (2026-08-19) to keep doctor green.
- Fix (2026-08-27): Effigy PR 45 admits inline `{ rhai = ... }` in the
  doctor schema; Longhorn restored `agent-control:install-skill` and doctor
  stays green (`ok:17 err:0`).
- Surface: `effigy doctor`, `effigy.toml` `[tasks]`.

### [x] Global `--repo` is consumed after `--` — 2026-08-27
- Friction: `effigy <task> -- --repo <PATH>` still switched catalogs, so
  consumer tasks could not receive `--repo` as their own argument.
- Fix (2026-08-27): Effigy PR 45 ends global flag parsing at `--`; verified
  `effigy agent-control:install-skill -- --repo <consumer>` reaches the Rhai
  task. Leading `--repo` before the task name still switches catalogs.
- Surface: `effigy --repo`, `agent-control:install-skill`.

### [x] Fresh worktree QA reaches TypeScript without installing dependencies — 2026-08-27
- Friction: `effigy qa` in a fresh worktree reached `check:ts` with no
  `node_modules`; `bun x tsc` fetched the compiler but left workspace imports
  unresolved.
- Fix (2026-08-27): `bootstrap:deps` runs `bun install --frozen-lockfile`;
  `qa` invokes it before TypeScript checks; `check:bun-deps` fails fast with
  the bootstrap selector when `check:ts` or `check:svelte` run alone.
- Not in scope: `deps link bun` replacing registry symlinks; greenfield proof
  path variables — separate papercut surfaces.
- Surface: `effigy qa`, `bootstrap:deps`, `check:bun-deps`, `check:ts`.

### [x] Prototype lockfiles go stale when a workspace crate gains a dependency — 2026-08-27
- Friction: adding workspace deps left `prototypes/gpui-*/Cargo.lock` stale
  until `effigy release gates` ran `check:prototypes --locked`.
- Fix (2026-08-27): Card 223 now records the ongoing lockfile-refresh step
  when workspace `Cargo.toml` deps change (`cargo update --offline
  --workspace` per prototype). Card 223's one-time refresh already landed
  2026-08-15; release gates still own the expensive `check:prototypes` lane.
- Surface: `prototypes/*/Cargo.lock`, Card 223, `check:prototypes`.

### [x] Repo-wide renames need to be language-aware — 2026-08-09
- Friction: `bovine` -> `split-shell` text substitution hit Rust identifiers
  (hyphens illegal); two crates stopped compiling.
- Fix (2026-08-11): `AGENTS.md` Working Posture — hyphen-free identifiers when
  the token is also a Rust name; `cargo check --workspace` before commit;
  two substitutions when a token is both string and identifier.
- Surface: `AGENTS.md`, rename practice.

### [x] Concurrent threads in one repository undo each other — 2026-08-09
- Friction: concurrent agents in one checkout; `git add -A` / index-based
  checks undid each other's moves silently.
- Fix (2026-08-11): `AGENTS.md` — stage by explicit path, never `git add -A`;
  verify moves in the working tree; prefer a branch per concurrent thread.
- Surface: `AGENTS.md`, multi-thread practice.

### [x] MSRV-gated Clippy lints surface late — 2026-08-06
- Friction: floor bumps unlocked Clippy on pre-existing code only when a
  release gate ran, not when the floor changed.
- Fix (2026-08-11): `release-baselines/rust-toolchains.env` and
  `scripts/README.md` require `effigy release:floor` in the same change as an
  MSRV bump, before commit. Gate already runs Clippy `-D warnings` at floor.
- Surface: `release-baselines/rust-toolchains.env`, `scripts/check-release-floor.sh`.

### [x] A public-readiness redaction disabled a release gate — 2026-08-10
- Friction: `6a84574c` replaced a consumer's real path with
  `../<private-consumer>` in `scripts/private-candidate-card149/consumers.ts`
  — executable code, not prose. The candidate verifier could not resolve it.
- Impact: g02.008 spent weeks recorded as "operator-held on nucleus
  quiescence" while the gate could not run at all.
- Fix: `1371a6dc` read the path from `LONGHORN_PRIVATE_CONSUMER` and recorded
  an unset value as a named omission. Lesson: redaction sweeps must not treat
  `scripts/` as prose. Card149 verifier surface later removed in `81f12053`.
- Surface: was `scripts/private-candidate-card149/consumers.ts`.

### [x] A receipt pinning five repositories goes stale silently — 2026-08-10
- Friction: Card 149's receipt pinned five external consumer graphs and
  Poodle's artifact set; drift accumulated silently until someone ran it.
- Fix / disposition (2026-08-11): verifier unstuck in `1371a6dc`; entire
  `scripts/private-candidate-card149/` surface removed in `81f12053`. Lesson
  retained on Card 149 — pin fewer things, or run often enough to fail early.
- Surface: was `scripts/private-candidate-card149/`, g02.008.

### [x] Candidate receipt freezes consumer graphs, coupling unrelated repos — 2026-08-06
- Friction: Card 127/149 receipt required clean selected manifests across
  seven consumers, so unrelated in-flight work blocked Longhorn freezes.
- Fix / disposition (2026-08-11): card149 consumer-graph surface removed in
  `81f12053`. Separate artifact-identity (Longhorn-only) from cross-repo
  compatibility if a successor receipt returns.
- Surface: was `scripts/private-candidate-card149/consumers.ts`, Card 149.

### [x] A `file:` install links files, so new files never reach consumers — 2026-08-10
- Friction: `bun install` for a `file:` dependency builds real directories
  containing one symlink per file, resolved at install time. Edits to an
  existing file are live through the link, but a file *added* to Longhorn
  afterwards has no link and simply does not exist in the consumer. Nucleus
  failed to launch on `Failed to resolve import "../generated/fields.ts"`
  while happily reading the edit that introduced that import — four of the
  eight generated field maps resolved and four did not, split exactly on
  whether they predated the install.
- Impact: adding any file to `packages/longhorn/src` silently breaks every
  consumer until it reinstalls, and the symptom points at the new file rather
  than at the install. Vite's optimized-dep cache holds the bad resolution
  too, so a plain reinstall is not always enough.
- Fix (2026-08-11): `docs/guides/getting-started.md` documents the trap and
  the portfolio path — from the consumer, `effigy deps link bun ../longhorn`
  (not raw `bun link`) so each package is a directory symlink and new files
  appear without reinstall. Re-link after `bun install`; unlink when done.
  Publishing by version also closes it (reinstall gets a complete tarball).
- Not: a consumer `postinstall` hook (fires at install time; breakage is later).
- Surface: `docs/guides/getting-started.md`, consumer `file:` deps.

### [x] Peered packages need a consumer override under `file:` refs — 2026-08-08
- Friction: `longhorn-poodle-svelte` and `longhorn-tauri` declare
  `@inflatable-cookie/longhorn` as a peer at `0.1.0`. A consumer that installs
  longhorn as `file:../longhorn/packages/longhorn` does not satisfy that peer
  by itself, so bun reaches for the registry and 404s. Nucleus and soundcheck
  happened to carry `overrides` already; jetstream did not and failed to
  install until one was added.
- Impact: every new consumer hits a confusing registry 404 for a package that
  is sitting on disk beside them, and the fix is not discoverable from the
  error.
- Fix (2026-08-11): `docs/guides/getting-started.md` now shows the
  `dependencies` + `overrides` pair for sibling `file:` installs, and states
  that the override is required until consumers depend by published version.
- Surface: `docs/guides/getting-started.md`, consumer manifests.

### [x] `check:bindings` cannot catch a generator that emits an undeclared type — 2026-08-10
- Friction: Card 177 added `SurfacePresentation` to the Rust surface model and
  regenerated. `packages/longhorn/src/surfaces/generated/protocol.ts` then
  *referenced* `SurfacePresentation` in four places while declaring it nowhere,
  because the generator's declaration list had not been extended.
  `effigy check:bindings` passed. It compares generated output against
  committed output, so when the generator is wrong both sides agree and the
  gate is satisfied by a file that does not compile.
- Impact: the natural loop after a protocol change — `generate:bindings`, then
  `check:bindings` — reports success on a broken package. `effigy check:ts`
  does catch it, so `qa` is not blind, but the bindings gate alone reads as
  authoritative and is not.
- Fix (2026-08-11): `longhorn-bindings` `apply()` now runs
  `assert_protocol_references_resolve` on every `protocol.ts` artifact before
  write/check. Referenced PascalCase names must resolve to a local
  `export type`, an `import type { … }` name, or a TypeScript builtin; comments
  and camelCase field interiors are ignored. Tagged-union field-map skips stay
  warnings — those types are declared, just not flat-mapped.
- Not: adding `check:ts` to `check:bindings`.
- Surface: `crates/longhorn-bindings/src/generation.rs`.

### [x] Installed skill copies go stale within one longhorn_version — 2026-08-19 (fixed 2026-08-20)
- Friction: the agent-control skill changed three times on 2026-08-19
  (command-less note, sanitized MCP names, listen ack distinction) while
  `longhorn_version` stayed 0.1.0. The drift check compares the stamp to
  the workspace version, so an installed consumer copy from the morning
  passes every gate while missing the day's fixes.
- Impact: consumer agents follow stale instructions; nothing tells the
  repo to re-run the installer.
- Plausible fix: add a content hash to the skill frontmatter, stamp it at
  install, and have the installer (or a consumer-side check) compare
  hashes; or simply bump `longhorn_version` policy to change-on-edit.
- Surface: `skills/agent-control/SKILL.md`, `install-agent-control-skill.ts`,
  `check:agent-control-skill`.
- Fix (2026-08-20): the installer compares trees on an equal version stamp
  and refreshes on any content difference ("refreshed ... — content
  changed"); fixture pins it. Bit Soundcheck first: their same-version
  re-run no-opped over the pre-fix copy and needed a manual delete.
