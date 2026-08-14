# 214 Port, Parity, And Keyring Coverage

Status: ready
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

## Acceptance Criteria

- [ ] all seven raw ports have conformance tests
- [ ] a malformed bridge-job event reaches `onFailure` and terminates the job
- [ ] settings navigation is pinned by the parity fixture
- [ ] the keyring contract suite runs on CI Linux against the mock
- [ ] poodle-svelte tests pass when run from the package directory

## Evidence Required

- the new suites, green
- the parity fixture diff
- `effigy qa` green

## Stop Conditions

Stop if settings-navigation parity reveals the two tiers actually diverge —
that is memo 022's shape and belongs in a contract conversation, not a test
edit.
