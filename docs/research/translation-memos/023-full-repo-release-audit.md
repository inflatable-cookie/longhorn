# 023 Full-repo Release Audit

Status: promoted — g02.021-027, cards 200-223 (2026-08-14)
Owner: Tom
Updated: 2026-08-14
Depends on: memo 018 (prior workspace-integrity audit); contracts 002, 004, 010, 017, 018, 019

## Prompt

Full audit ahead of release hardening: security, complexity, DX, UX of the
developer surface, flaws, gaps, poor practice. Nine parallel audit lanes over
every crate, package, script, workflow, and the docs spine. Findings below are
verified against the working tree at `a99d3abb`; headline items were
re-verified by a second reader against the code (extraction, endpoint parsing,
Poodle link state, published tarball contents, `POODLE_REPO` throw, docs
drift).

## Method

Nine read-only audit lanes: (1) crypto/credentials, (2) update supply chain,
(3) IPC/bridge/command, (4) state/data crate quality, (5) presentation/platform
crate quality, (6) TypeScript packages, (7) DX/tooling/automation,
(8) docs-vs-code drift, (9) hygiene/dependencies/repo shape. Plus `effigy
doctor`, a live `cargo deny check advisories` run, and a download of the
published `@inflatable-cookie/poodle-svelte@0.1.0` tarball.

## Verdict

The engineering core is genuinely strong: type-forced artifact verification,
per-caller bridge authorization, zero reachable panic on untrusted input across
~300 traced panic paths, strict TypeScript with zero suppressions, uniform
test suites per crate. The release risk is not in the systems — it is in the
*evidence and envelope around them*: gates that pass against a linked sibling
instead of the published peer, a docs spine that still specifies a deleted
architecture (Card 179), and a coverage hole where the layout test suites were
deleted without being ported. All three are roadmap-shaped, not code-shaped.

## Critical

**C1. All local TS gates pass against a linked sibling Poodle, not the published
peer the release pins.** `node_modules/@inflatable-cookie/poodle-svelte` is a
bun global link into the live sibling checkout
(`/Users/tom/Dev/projects/poodle`). `packages/longhorn-poodle-svelte` publicly
exports `./settings/poodle` and `./update/poodle`, whose sources import
`SettingsShell`, `UpdateCenter`, `UpdateStatus` from
`@inflatable-cookie/poodle-svelte` (`src/settings/poodle/SettingsShell.svelte:7`,
`src/update/poodle/UpdateCentre.svelte:2`, `src/update/poodle/UpdateStatus.svelte:2`).
The published `0.1.0` tarball — downloaded and listed — contains none of
those components; the declared peer range is exactly `"0.1.0"`
(`packages/longhorn-poodle-svelte/package.json:155`). So `check:ts`,
`check:svelte`, `test:vitest`, and every proof are green only because of the
link. `verify-repo-containment.ts` scans manifests for `file:`/`link:` specs
and cannot see bun's global link state. `poodle-release.ts:129-134` claims to
verify "the copy actually installed in node_modules" but compares only the
version string. `LONGHORN_PROOF_ACCEPT_LINKED_POODLE=1` is hardwired into
`proof:artifacts` (`effigy.toml:68`) and `linkedPoodleAccepted: true` is
recorded (`verify-settings-composition-proof.ts:56`) but **nothing fails a
release on it**. `ci-rehearse.sh:40-57` resets `CARGO_HOME` but not bun link
state. Net: every gate green → `release.yml` publishes a package whose public
settings/update surfaces fail to resolve against the pinned peer.

**C2. The docs spine still specifies the architecture Card 179 deleted.**
Card 179 (`dfa72456`, `a4dda1f7`, 2026-08-10/11) removed `LayoutContainerId`
and deleted `longhorn-layout`/`longhorn-layout-config` into
`longhorn-surfaces`/`longhorn-surfaces-config`. The code moved; the canonical
docs did not:

- `docs/architecture/system-architecture.md:19-37,74-93,90-93,160` — hosting
  model and entire "Layout core" layer built on the opaque layout container;
  names both deleted crates as implementers. `LayoutContainerId` has zero
  matches in `crates/` or `packages/`.
- `docs/contracts/002-*.md` is self-contradictory: the absorbed 2026-08-11
  section was appended; the pre-absorption body (`:17-31,42,93,339`) still
  chains `WindowId -> LayoutContainerId` and claims Nucleus composes
  `longhorn-layout` without `longhorn-surfaces`. `:421` points layout
  substance at contract 014, whose stub points back at 002 — a pointer loop.
  Acceptance criterion `:411` ("no Surface package") is falsified by
  `examples/nucleus-no-surface-proof/Cargo.toml:11`.
- The generated `api-surface.md` is fully current — the handwritten canonical
  docs are the drift.

## High

**H1. Archive extraction is not bounded: symlink and hardlink entries escape
the destination.** `crates/longhorn-update-install/src/lib.rs:194-233` checks
only entry *names* via `bounded()` (`:240-259`), then calls
`entry.unpack(&destination)` — tar's `unpack` skips `validate_inside_dst` (that
guard exists only in `unpack_in`). Link targets are never validated: a
two-entry archive (`link -> /target/dir`, then `link/payload`) passes
`bounded()` and writes outside staging. The crate header (`lib.rs:34-36`) and
contract 018:51 claim the opposite. Precondition holds honestly: the artifact
is minisign-verified first, so this needs the signing key — but the stated
property the design relies on ("a signature proves origin, not good intent",
`lib.rs:238-239`) is false for link entries, and the existing escape test
(`tests/install.rs:192-211`) covers only textual `../`.

**H2. `EndpointUrl` loopback check bypassable with userinfo.**
`crates/longhorn-update/src/source.rs:51-63` splits the authority without
stripping `user@`: `http://127.0.0.1:80@evil.example/x` and
`http://[::1]@evil.example/` both parse as loopback and are accepted — the
fetch goes to a remote host over plaintext. Breaks the documented load-bearing
gate ("HTTPS is required, with one exception", `source.rs:9-14`). Tests cover
prefix tricks, not userinfo.

**H3. The `private-candidate` release gate hard-fails without an undocumented
`POODLE_REPO`, from an import-time side effect in code it never uses.**
`scripts/verify-private-candidate-docs-card127.ts:3` imports
`private-candidate-card127/support.ts`, whose module-level `poodleRoot` throws
when the env var is unset (`support.ts:9-16`; verified live — gate fails in
16ms). The script's assertions read only `fixtures/`, `docs/`, `CHANGELOG.md`.
`scripts/README.md:44` claims no script takes a `*_REPO` override. Any release
run on a machine without this undocumented var stops dead.

**H4. `check:runner-tools` is wired into nothing automated.** It lives only in
the manual `ci-rehearse.sh:38`; absent from `qa`, `release:gates`,
`[release.gates]`, and both workflows. The guard against the failure class
that killed release run 6 (missing `rg`) can regress silently.

**H5. Docs front doors and registers are stale on their load-bearing claims.**

- `docs/README.md:32-33` and `docs/reference/private-0-1-candidate.md:41-42`:
  "18 TypeScript packages and 41 Rust crates" — reality is 3 and 45 (Card 164
  consolidation; `Cargo.toml:3-47`). Wrong twice, handwritten both times.
- Held-surface register (`docs/reference/held-surface.md`, self-described
  "single source of truth") stale on 3 of 7 rows: update (`:31` — renderer
  surface exists since g02-154), licensing (`:32` — TS surface exists since
  g02-158), replay (`:35` — `apply_with_replay` has *zero* tests or callers
  repo-wide; the claimed contract tests were deleted with `longhorn-layout`).
- `docs/guides/package-selection.md`: the "Add One System At A Time" table is
  duplicated (`:79-95`, `:112-128` — merge artifact), both copies select the
  deleted `longhorn-layout`, and the copy-paste manifest (`:50-59`) lists a
  subpath as an installable dependency. A consumer following this guide cannot
  install.
- `docs/roadmaps/generation-index.md` contradicts itself: three competing
  "live next step" pointers (`:105`, `:121-122`, `:136-137`), and "publication
  is no longer deferred" (`:106-107`) vs "publication remains deferred"
  (`:119`) in the same file. Not updated for the 2026-08-14 card closes.
- `docs/guides/getting-started.md:95-101` installs the 17-tarball layout frozen
  in the Card 127 receipt; the tree produces 3. No working TS install path is
  documented.

**H6. The card127 candidate receipt misdescribes the commit it freezes, and the
gate locks the error in.** `fixtures/release/card127/private-0-1-candidate-v1.json`
lists 36 Rust entries with only 34 unique (`longhorn-surfaces` ×3,
`longhorn-surfaces-config` ×2); `longhorn-layout`/`longhorn-layout-config`,
which existed at frozen commit `7a8cc7b4`, are absent. TS list similarly
duplicated, and package names postdate the rename — the receipt was
regenerated after the fact and mislabeled as of that commit.
`verify-private-candidate-docs-card127.ts:34-36` enforces counts that only
hold because of the duplicates. The hash chain means the receipt cannot be
honestly regenerated; annotate, don't rewrite.

## Medium

### Security — credentials & update

- **Account token interpolated unescaped into redemption JSON.**
  `crates/longhorn-licence/src/activation.rs:277-283` builds the request body
  with `format!`; a token containing `"` or `\` yields malformed JSON or field
  injection. Same for `activation_id` in renew/release. One `serde_json::json!`
  fixes it; no test covers JSON metacharacters.
- **Bridge events are app-wide broadcasts; read authority not enforced on the
  event channel.** `TauriBridgeEventSink::emit` uses `app.emit`
  (`crates/longhorn-tauri-bridge/src/events.rs:43`);
  `publish_domain_event` checks session + epoch, not read authority
  (`handler/publication.rs:20-35`). Every webview receives full typed payloads
  for all sessions, including windows with `ReadAuthority::None`; client-side
  filtering is advisory. No `emit_to` anywhere. Asymmetric hole in an otherwise
  per-caller model; needs a recorded decision (hint-only events or per-window
  targeting).
- **Bridge sessions never die.** No close/teardown path; a destroyed window's
  session stays valid and the map grows per distinct caller label
  (`crates/longhorn-tauri-bridge/src/handler.rs:99-107`). Session-id entropy is
  consumer-supplied and undocumented (fixtures use predictable ids).
- **The update swap is not crash-atomic and the backup has no recovery path.**
  `crates/longhorn-update-install/src/lib.rs:150-180`: rename target→backup,
  rename staged→target, real window where the app does not exist; nothing
  restores `*.longhorn-previous` on next launch; next swap deletes the backup
  first.
- **No download size limit anywhere; full artifact buffered pre-verification.**
  `ArtifactFetch::fetch` returns unbounded `Vec<u8>`
  (`crates/longhorn-update/src/fetch.rs:76-87`); decompression likewise
  unbounded. Contract 018:70 declares the artifact host untrusted
  infrastructure; size is the one unverified resource dimension.
- **`PrivilegedReplace` installs a tree mutable after verification.** Verified
  archive extracts to user-writable staging; escalated move happens later over
  possibly-modified content (`install/lib.rs:125-137,182-186`). No host
  implementation exists yet — write the re-verification contract before the
  first one.
- **Loopback listener aborts the whole sign-in flow on a failed answer to a
  non-callback probe.** `crates/longhorn-browser/src/loopback.rs:113-132` —
  contradicts the module's own contract ("keeps waiting", `:23-24`). A local
  scanner RSTing before the 404 kills the wait. Related: deadline not enforced
  mid-connection; a byte per <5s dribble holds the flow open (`:142-165`).

### Quality & complexity

- **`#[allow(missing_docs)]` on the most public surface.** Wire types exported
  to TypeScript opt out of the workspace docs deny:
  `longhorn-operation/src/protocol/mutation/receipt.rs:55,190`,
  `result.rs:13`, `longhorn-notifications/src/protocol/mutation.rs:11,145,264`,
  `snapshot.rs:108`.
- **`Display` via `{self:?}` in history-tree/config**, inconsistent with
  hand-written sibling messages; strings reach host adapters and couple to
  internal variant names (`history-tree/src/error.rs:168`,
  `checkpoint.rs:251`, `retention.rs:596`; `config/.../error.rs:92`).
- **The native-content generation discipline is implemented three times and
  already diverging.** Near-identical `validate_plan` in all three mechanism
  adapters; backing-surface alone checks `invalidated_generation`,
  isolated-window alone has `FailedGeneration` re-attach. Contract 017 states
  the rule once; a fix must now land three times in three shapes.
- **The GPUI host seam is never compiled against real GPUI in the workspace.**
  `longhorn-gpui-windowing` is a pure adapter; the only real binding is the
  workspace-excluded prototype. Adapter assumptions drift-detect only by hand.
- **TS boundary test's domain list is stale.** `packages/longhorn/tests/
  boundary.test.ts:12-28` omits `licence` and `update`; a new domain can be
  added without the boundary test noticing — exactly the drift it exists to
  catch.
- **No automated binding between hand-written invoke/event strings in
  `longhorn-tauri` and Rust command registrations** (e.g.
  `packages/longhorn-tauri/src/licence.ts:11-17` vs
  `crates/longhorn-tauri-licence/src/commands.rs:13`). Tests use the exported
  constants on both sides of the fake transport, so a typo passes everything.
  The last un-gated hand-written seam.
- **`listenTauriBridgeJob` drops malformed events with no failure channel**
  (`packages/longhorn-tauri/src/bridge-events.ts:78-101`) — the job never
  terminates for the consumer; sibling `CheckedSnapshotConnection` records and
  tears down.

### DX, tooling & automation

- **`ci.yml` hand-transcribes gates and has drifted**: 13 binding domains vs 15
  (`ci.yml:93-98` vs `effigy.toml:53` — missing `licence`, `update`);
  svelte-check threshold laxer than local (`--threshold error` vs default);
  MSRV hardcoded while `release.yml` reads `release-baselines/`.
- **The MSRV floor has ~15 hardcoded copies across proof scripts**; nothing
  derives them from `rust-toolchains.env` or cross-checks `Cargo.toml:63`.
  `LONGHORN_CURRENT_STABLE` is read by nothing.
- **Duplicate release-gate definitions** (`config/release.toml:41` vs
  `effigy.toml:141` both define an `effigy qa` gate; `[release]` keys duplicated
  across both files); qa runs twice in release prepare; gate order runs the
  45s floor before the 16ms env-dependent check.
- **`docs:rust` is in no gate** — not qa, not CI, not release.
- **`health` (doctor) is not cheap**: maps to `release:floor` (two clippy
  passes + full test suite at MSRV), contradicting its own comment and
  AGENTS.md's "cheap health".
- **Effigy version split-brain**: `release.yml:95-97` pins setup-effigy 0.9.1;
  local is v0.11.0+local. Release evidence and local qa are no longer the same
  binary.
- **Pack-path divergence**: qa packs with `bun pm pack --dry-run`; release
  packs/publishes with `npm pack`/`npm publish` (`release.yml:176,218`).
- **No dependency-advisory gate.** Live `cargo deny check advisories` fails on
  13 advisories, all `unmaintained`-class transitive via Tauri (GTK3 via wry,
  `unic-*` via urlpattern, proc-macro-error). Zero vulnerability-class today;
  nothing would surface a future one.
- **`licence` and `update` protocols have no golden fixture**; the 13 older
  domains all do (`crates/longhorn-bindings/src/licence.rs`, `update.rs` lack
  `GOLDEN_FIXTURE`). The newest protocols lack the neutral cross-language
  fixture.

## Low (selected; full detail in lane notes)

- `Debug` on secret-carrying licence types prints credentials in full
  (`activation.rs:125`, `account.rs:69,22`, `protocol.rs:306`) — latent, no
  logging today, but contrasts with config-age's deliberate redaction.
- `verify()` records attacker-controlled `key_id` unbound to the verifying key
  (`longhorn-licence/src/verify.rs:67-70`).
- `authority_epoch` is constant for a controller's lifetime; `StaleAuthority`
  can never fire (`longhorn-update/src/controller.rs:84`).
- `evaluate` never checks `manifest.channel == build.channel`.
- Predictable staging/backup names pre-creatable in a shared dir
  (`install/lib.rs:125,151`); cleanup is best-effort with no sweep.
- Tamper-after-header-MAC misclassified as `Locked` instead of documented
  `Corrupt` (`longhorn-config-age/src/inspection.rs:184-194`).
- Hand-rolled hex encoding duplicated six times with three error idioms.
- Six near-identical Tauri adapter error scaffolds with drifting code names.
- `json_string` escapes only `"`, `\`, `\n` in the bindings generator
  (`generation.rs:397-410`); misplaced doc comment at `:95-105` invisible
  because the crate is a binary and `missing_docs` never fires.
- Dead `svelte-shims.d.ts` files in the peerless package; `disposeTerminal`
  rejection leaks the progress listener (`bridge-events.ts:107-110`);
  poodle-svelte test paths are cwd-relative.
- 20 committed generated Tauri schema files (~1.15 MB) with no freshness gate;
  CI floats on `stable`; redundant workspace `exclude` entries
  (`Cargo.toml:54-57`); unpinned actions and `npm install -g npm@latest` in a
  publish-rights step (`release.yml:60-62,108`).
- Direct deps a major behind latest: `keyring` 3.6.3, `ed25519-dalek` 2.2.0,
  `ts-rs =11.1.0`, `base64` 0.22.1; `=tauri-build 2.6.3` pin dates to the
  initial commit and produces a mixed Tauri stack; exact pins carry no
  rationale comments.
- `scripts/README.md` stale (twelve vs thirteen proof scripts, names a deleted
  module, false `*_REPO` claim); `generate-api-reference-card126.ts:39` tells
  users to run a task that does not exist and hardcodes `0.1.0`.
- Doctor errors today: 40 god-file findings (mostly test files, prototypes,
  and `bindings/generation.rs`) and one attention-marker false positive
  (`decision.rs:142` comment contains `[SECURITY]`).

## Coverage Gaps (ordered by release risk)

1. **Post-179 layout test-suite loss — the biggest single hole.** Card 179
   deleted the `longhorn-layout` suites (definitions, donors,
   mutation/policy/replay, state, visibility — 12 files) and the 321-line
   `longhorn-layout-config` debounce suite. Today `crates/longhorn-surfaces/
   src/layout/**` has zero inline tests; `tests/surface_contract/` covers
   Surface topology but no panel-mutation commands; `longhorn-surfaces-config`
   has no debounce or replay tests. The deleted behavior is exactly what
   contract 002's absorbed sections specify. `apply_with_replay` /
   `BoundedLayoutReplayStore` are dead code contradicting the held-surface
   register.
2. **No fuzz/property testing** despite three hand-rolled untrusted-input
   parsers: the zip backup inspector (`longhorn-config/src/backup/archive/`),
   `parse_utc_timestamp` (one strictness test), history/history-tree envelope
   decoders.
3. **No end-to-end ACL enforcement test**: capability templates are
   file-content drift tests; whether a denied window is actually refused a
   mutate command is delegated to Tauri, untested.
4. **Extraction tests lack hostile link fixtures**; no userinfo URL cases; no
   mid-swap crash or backup-recovery test; no size-limit test possible (no
   limit exists). No production `ArtifactFetch` implementation exists in-repo
   to audit — the real one is consumer-side and unwritten.
5. **5 of 7 `longhorn-tauri` raw ports untested** (`history`, `history-tree`,
   `licence`, `notifications`, `update`) — the untested part is exactly the
   hand-written command/event strings.
6. **Settings navigation projection has no cross-language parity fixture**
   (581-line module; the crate's own doc says the TS port "is the port, not
   the source"); config and licence projections likewise unpaired.
7. **Keyring contract tests run only against real macOS/Windows keychains** —
   nothing exercises them on CI Linux; no mock-backend test of
   `Unavailable`-vs-`None`.
8. **`scripts/` has zero tests** — all verification logic is unverified except
   by running it.
9. **No redaction test** asserting `Debug` of licence types contains no secret;
   no JSON-metacharacter test for activation payloads; no loopback
   disconnect-probe or trickle-past-deadline test.
10. **8.2k-line bindings generator** has unit tests in only 3 files; fixture
    renderers rely on the byte-diff gate the generator itself admits cannot
    catch a self-consistent broken generator (the
    `assert_protocol_references_resolve` backstop partially compensates).

## Verified Clean (so the roadmap does not re-litigate)

- Update verification is type-forced: `VerifiedArtifact` constructible only by
  `verify_artifact`; `PublicKey::verify(..., false)` rejects legacy formats;
  no downgrade by version comparison (`AheadOfChannel`, exact-match offer).
- Bridge authorization is per-caller and layered: caller identity from
  `window.label()`, cross-caller session use rejected, per-route
  session/capability/authority checks, stale-epoch rejection, strict bounded
  ASCII id grammars, `deny_unknown_fields`, exact protocol-version rejection.
  Keymap commit re-validates digest-bound evidence against fresh state.
  Zero shell/process usage in the IPC surface; no path input accepted.
- Zero reachable panic on untrusted input across ~300 traced unwrap/expect
  sites in state/data crates — the `expect("validated …")` invariant idiom is
  pervasive and holds.
- Licence: verify-before-parse with `verify_strict`, signature over raw bytes,
  constant-time state check, fail-closed loopback (ephemeral bind, 8 KiB head
  bound, probes 404), 0600 on persisted configs, redacted `Debug` on age
  secrets, HTTPS-only activation, capability test pinning activation as its
  own Tauri permission.
- TypeScript: `strict` everywhere, zero `@ts-ignore`/`@ts-expect-error`, every
  bridge cast validate-then-cast, licence key parser bound to Rust via a
  generated conformance fixture.
- Repo shape: `node_modules`/`target`/`private/` untracked; both lockfiles
  committed; all 45 crates are members; all 16 fixture trees referenced;
  crates all `publish = false`; npm trusted publishing with no stored secrets;
  LICENSE matches.
- Prototypes do not bleed into shipping crates; macOS-only gating matches
  contract 017's recorded decision; large files (`lifecycle.rs`,
  `generation.rs`, `retention.rs`) are internally organized, not god-modules
  in the harmful sense.

## Roadmap Translation (candidate g02/g03 milestone clusters)

Ordered by release risk. Each cluster is card-sized work; severities above map
to cluster order.

1. **Release-evidence integrity** (C1, H3, H4): fail release gates when
   `linkedPoodleAccepted` is true; pack-level typecheck of
   `longhorn-poodle-svelte` against *registry* Poodle; teach `ci-rehearse`/
   containment about bun global links; lazy `poodleRoot` + document
   `POODLE_REPO`; wire `check:runner-tools` into `qa`.
2. **Supply-chain hardening** (H1, H2, M-swap, M-size, M-PrivilegedReplace):
   validate link targets or reimplement over `unpack_in` semantics with hostile
   fixtures in the shared conformance suite; parse endpoint authorities
   properly (userinfo); startup recovery sweep for `*.longhorn-previous`;
   `max_bytes` on fetch + extraction quota; write the `PrivilegedReplace`
   re-verification contract; close the Card 196 downgrade decision via
   minisign trusted comment.
3. **Docs spine reconciliation** (C2, H5, H6): rewrite
   `system-architecture.md` hosting/layout sections; revise contract 002's
   body and break the 002↔014 pointer loop; resolve the 002 acceptance
   criterion vs `nucleus-no-surface-proof` (rename or restate); single-source
   the package/crate counts (generate, as `api-surface.md` already is);
   held-surface freshness gate tied to roadmap state; generation-index
   hygiene (one next-task field, updated in the closing commit); fix
   `package-selection.md` and `getting-started.md` install paths; annotate
   the card127 receipt discrepancy.
4. **Coverage restoration** (gaps 1-7): port the deleted layout suites onto
   `longhorn-surfaces`/`-config`; wire up or delete `BoundedLayoutReplayStore`;
   golden fixtures for licence/update; fuzz gate for the three untrusted
   parsers; ACL enforcement proof app; tests for the 5 untested tauri ports;
   settings-navigation parity fixture; keyring mock-backend tests.
5. **Bridge & session lifecycle** (M-events, M-sessions): recorded decision on
   event scoping (`emit_to` vs hint-only contract); session teardown wired to
   window destroy; documented unguessable-session-id requirement; pre-parse
   IPC byte caps on mutation commands; harden the two packaged proofs
   (strict CSP, drop `withGlobalTauri`, scoped permissions).
6. **Credential hardening pass**: `serde_json::json!` for redemption bodies;
   `Zeroizing<String>` on the credential path; redaction pass over licence
   `Debug` impls + redaction tests; CSPRNG `CodeVerifier::generate()`; a
   Longhorn-owned age-identity persistence slot; `key_id` binding decision;
   payload size bound on `SignedLicence`.
7. **Automation convergence** (M-ci, M-MSRV, M-dup-gates, M-docs:rust,
   M-effigy-version, M-pack-path, M-advisories): route `ci.yml` through effigy
   selectors; single-source the MSRV from `rust-toolchains.env`; collapse
   duplicated `[release]`/`[release.gates]`; add `docs:rust` to a gate; pin
   release-runner effigy in lockstep; converge on one pack tool; add
   `deny.toml` + advisory gate with the 13 known unmaintained advisories
   explicitly allowed and dated.
8. **Structural consolidation** (M-native-content, M-GPUI, low-tier quality):
   hoist the attach-generation state machine into `longhorn-native-content`;
   gated real-GPUI build; shared hex helper in `longhorn-core`; adapter
   error-scaffold convergence; codify the panic-invariant idiom in contracts;
   dependency sweep (keyring 4, ed25519-dalek 3, ts-rs 12, base64 0.23,
   `=tauri-build` pin rationale) with the bindings-generator fragility note
   attached to any ts-rs unpin.

## Audit Limits

- Static review plus gate observation; no dynamic exploitation, no fuzzing
  executed (fuzz targets are named as a gap, not exercised).
- Consumer repos were not audited (read-only cross-project posture); the
  linked-Poodle finding was verified from this side only.
- `cargo deny` was run with default config (no `deny.toml`); license/ban
  checks unconfigured.
- Prototypes were sampled, not read end-to-end; they are workspace-excluded
  and evidence-only per Card 198.
