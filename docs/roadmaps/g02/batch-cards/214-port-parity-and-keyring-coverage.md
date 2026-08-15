# 214 Port, Parity, And Keyring Coverage

Status: complete
Completed: 2026-08-15
Owner: Tom
Roadmap: g02.024 batch 4
Governing refs: contract 010; contract 013; contract 019; memo 023 (gaps 4-7,
10; TS-M3, TS-L2/L3)
Depends on: Card 212 (the seam-string check shapes how the port tests assert)
Auto-start next card: no

## Objective

The thin-but-untested surfaces get uniform coverage: five `longhorn-tauri`
raw ports, the settings-navigation parity gap, bridge-job failure handling,
keyring off-platform behavior, and the bindings generator's untested lanes.

## Why this exists

- 5 of 7 raw ports in `packages/longhorn-tauri` (`history.ts`,
  `history-tree.ts`, `licence.ts`, `notifications.ts`, `update.ts`) have no
  tests anywhere; only `native-content` and `operation` get cross-package
  conformance tests.
- The settings navigation projection — `longhorn-poodle`'s largest module
  (581 lines), whose own doc says the TypeScript `projectSettingsRegistry`
  "is the port, not the source" (`crates/longhorn-poodle/src/settings.rs:9-13`)
  — has no cross-language parity fixture; `fixtures/parity/projection-v1.json`
  covers notifications, operation, and update-restore only.
- `listenTauriBridgeJob` (`packages/longhorn-tauri/src/bridge-events.ts:
  78-101`) parses events inside the transport callback with no failure
  channel — a malformed terminal event means the job never terminates for the
  consumer. Sibling `CheckedSnapshotConnection` records and tears down. Its
  unlisten path also leaks the progress listener if terminal disposal rejects
  (`:107-110`).
- Keyring contract tests run only against real macOS/Windows keychains; CI
  Linux exercises nothing, and the `Unavailable`-vs-`None` distinction has no
  mock-backend test.
- The 8.2k-line bindings generator has unit tests in 3 files; fixture
  renderers rely on the byte-diff gate the generator itself admits cannot
  catch a self-consistent broken generator.
- `packages/longhorn-poodle-svelte` test paths are cwd-relative
  (`tests/boundary.test.ts:7-8`); the other packages anchor on
  `import.meta.url`.

## Scope

- `packages/longhorn-tauri` — port tests, bridge-job failure channel
- `fixtures/parity/` — settings navigation (config and licence if cheap)
- `crates/longhorn-credential-keyring` — mock backend
- `crates/longhorn-bindings` — generator unit coverage
- `packages/longhorn-poodle-svelte` — path anchoring

## Steps

1. Port tests for the five uncovered raw ports, following the
   `native-content`/`operation` conformance shape.
2. Add an `onFailure` channel to `BridgeJobListeners` mirroring
   `ConnectionFailureReporter`; malformed-event tests; `Promise.allSettled`
   (or try/finally) in unlisten.
3. Extend the parity fixture to settings navigation; add config and licence
   projections if the same shape covers them cheaply.
4. Mock-backend keyring tests: the contract suite runs against a mock on any
   platform; `Unavailable`-vs-`None` asserted off-keychain.
5. Generator coverage: unit tests for the fixture renderers' divergence-prone
   paths; consider lib+bin so `missing_docs` applies (fixes the misplaced
   doc-comment class structurally — coordinate with Card 222, which owns the
  `json_string` fix).
6. Anchor poodle-svelte test paths on `import.meta.url`.

## Do Not

- Test through the fake transport in a way that uses the same constant on
  both sides — Card 212's conformance check is what makes these tests mean
  something.

## Result

- All five raw ports have conformance tests (10 new tests) asserting literal
  command/event strings — pinned independently of the port constants, which
  `check:tauri-seam-strings` pins against Rust. The two layers make the same
  typo fail twice for different reasons.
- `BridgeJobListeners` gains `onFailure` (reusing `ConnectionFailureReporter`):
  a malformed event reports once, tears both listeners down, and terminates
  the job instead of hanging it. The unlisten leak is fixed with
  `Promise.allSettled` behind one `close()`.
- Settings navigation joins the parity fixture (3 cases pinned on the
  grouping rule both tiers agree on), Rust and TS halves both passing.
- Keyring error mapping is extracted into a shared module with a mock backend
  proving `Unavailable`-never-`None` off-keychain; the real-keychain tests
  still run on macOS/Windows.
- poodle-svelte paths anchor on `import.meta.url`; vitest runs green from the
  package directory now (137/137), not just the root.

**Divergence found, not edited:** Rust `sidebar_nav`
(`crates/longhorn-poodle/src/settings.rs:86-116`) prefixes section labels with
the module label in multi-module sidebars and its doc claims to mirror
`SettingsShell.svelte` exactly — but the Svelte side deliberately removed that
prefix (`SettingsShell.svelte:100-105`). The two tiers disagree on
multi-module sidebar labels and memo 022 does not record it. The parity
fixture pins only where they agree; the label question is a contract-013
conversation for the operator, not a test edit.

**Resolved 2026-08-15** (operator): the Svelte side is correct — the
section's own label, always. The Rust prefix is removed; both tiers pin the
rule by test; memo 022 carries the addendum.

## Acceptance Criteria

- [x] all seven raw ports have conformance tests
- [x] a malformed bridge-job event reaches `onFailure` and terminates the job
- [x] settings navigation is pinned by the parity fixture (grouping rule;
  label divergence recorded above)
- [x] the keyring contract suite runs on CI Linux against the mock
- [x] poodle-svelte tests pass when run from the package directory

## Evidence Required

- the new suites, green
- the parity fixture diff
- `effigy qa` green

## Stop Conditions

Stop if settings-navigation parity reveals the two tiers actually diverge —
that is memo 022's shape and belongs in a contract conversation, not a test
edit.
