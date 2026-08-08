# 018 Workspace Integrity Audit

Status: complete and promoted  
Owner: Tom  
Updated: 2026-08-03  
Promotes: the g02 remediation runway. No new contract; every finding sits
inside contracts 004, 010, 011, 012, 014, and 017.

## Prompt

Audit the whole workspace after g01 closeout: code flaws, doc drift, QA
surface gaps, and downstream consumer exposure. Decide what remediation work
the next generation must carry.

## Sources

Full-workspace static review at `03a654ba` (clean tree): all 38 crates and 18
packages, `effigy.toml`, docs spine, plus read-only consumer scans across
nucleus, loophole (aura/pulse/echo), soundcheck, figmatic, finch, jetstream,
and kimi-shell. `cargo check --workspace` clean; all TS test surfaces green.

## Findings

### Layout sizing integrity (contract 014)

- `crates/longhorn-layout/src/ratio.rs:9` — derived `#[serde(transparent)]`
  `Deserialize` bypasses the `from_millionths` ≤1.0 invariant. Any serde
  source (documents, mutation commands, definitions) can materialize a
  forbidden ratio. `longhorn-core/src/scale.rs` already models the correct
  validating pattern.
- `crates/longhorn-layout/src/definition/validation.rs:84` — `validate_schema`
  never caps `maximum <= LayoutRatio::ONE`; combined with the above, layout
  space over-allocates past 100% through registry-valid documents.

### Window lifecycle correctness (contract 017 host edge, contract 009)

- `crates/longhorn-tauri-windowing/src/lifecycle/host/directives.rs:120` —
  `Flush` executes synchronously inside `on_window_event`; blocks the event
  loop up to `flush_timeout` on every `Destroyed` and user-close-with-pending.
  Shutdown defers via `pending_flushes`; the event path does not.
- `crates/longhorn-tauri-windowing/src/lifecycle/host.rs:156` —
  `retag_window` renames only the host map; coordinator state stays under the
  old id: orphaned pending capture/flush, process-lifetime leak, capture
  generation reset.
- `lifecycle/host.rs:102` — `install_window` panics on >256-byte labels after
  map insertion; public entry, reachable, leaves listenerless state.
- `lifecycle/services.rs:108` — scheduler parks one blocking-pool thread per
  pending wake via `thread::sleep`; wakes fire on every `Moved`/`Resized`,
  are non-cancelable, and delivery failures at `services.rs:114` are
  swallowed (`let _ =`) despite the `WindowLifecycleReporter` seam.
- Low races: reveal-retry loss (`host/reveal.rs:83`), `retained_normal`
  lost-update (`host/directives.rs:52` with `host.rs:344`), destroyed-entry
  resurrection (`host.rs:204` with `coordinator.rs:115`), first-failure
  aborts multi-window reveal (`host/reveal.rs:54`), poisoned registry
  permanently `Busy` with no drop guard (`composition/host.rs:82`), dynamic
  windows hardcode `initial_normal: None` so maximize-before-first-capture
  never persists (`composition/assembly.rs:37`).

### Transfer session truthfulness (contract 011)

- `crates/longhorn-surface-transfer/src/commit/existing.rs:47` and
  `commit/provisioned.rs:43` — post-consumption binding/load failures return
  without `.consumed()`; the abort serializes `session_consumed: false` while
  the coordinator marked the session `Attempted`. The panel path
  (`longhorn-transfer/src/panel/commit.rs:60`) is correct; asymmetry.
- `commit/existing.rs:82`, `commit/provisioned.rs:130` — post-publication
  `assert_eq!` is a reachable release panic after durable Surface commit where
  every comparable failure returns `HostReconciliationRequired`.
- `crates/longhorn-tauri-transfer/src/handler.rs:76` — snapshot probes before
  the state lock; destroy in between re-inserts a client binding nothing
  removes; repeated cycles exhaust `maximum_client_windows`.
- `crates/longhorn-tauri-transfer/src/commands.rs:179` — client-changed event
  emitted outside the lock; out-of-order delivery strands stale epochs until
  re-snapshot; emit failure hides an already-advanced epoch behind `Err`.

### Host-thread and storage coordination (contracts 004, 010)

- Sync `#[tauri::command]` handlers across `longhorn-tauri-config`,
  `-settings`, `-command` do fsync-heavy IO on the main thread;
  `crates/longhorn-config/src/coordination.rs:230` spin-waits the file lock
  in a 2 ms `thread::sleep` loop up to the caller timeout. Contended lock or
  storage migration freezes the UI.
- `crates/longhorn-config/src/backup/restore/execution.rs:240` — best-effort
  journal cleanup after success; on failure, bare `ConfigStore::load` returns
  `Unavailable(RestoreActive)` forever; only `with_coordinated_load_set`
  self-heals via `recover_guarded`.
- `coordination.rs:189,278` — poison discarded via `into_inner()` while the
  rest of the workspace surfaces typed `Poisoned` errors.

### Observability

No `log`/`tracing` anywhere; ~25 `let _ =` swallows (event emits, adapter
teardown, wake delivery, journal cleanup) are invisible. Individually
best-effort; systemically there is no diagnostic seam.

### QA surface and packaging (contract 012)

- ~~`effigy.toml` history persistence selectors reference missing
  fixtures~~ — retracted during g02.006: the paths are crate-relative
  (`crates/longhorn-history/fixtures/history/linear-v1.json`,
  `crates/longhorn-history-tree/fixtures/history/tree-v1.json`), exist, and
  both selectors pass.
- `check:bindings-card127` omits `check:history-tree-bindings`;
  `check:client-ts` omits a `check:layout-ts` task.
- ~~`packages/svelte` svelte peer range diverges from the workspace
  convention~~ — retracted during g02.006: the `<=5.56.8` cap is deliberate
  proven-upper-bound policy, pinned by the package test and
  `docs/guides/compatibility-and-upgrades.md`; `packages/bridge` hard-deps `@inflatable-cookie/longhorn-tauri` where
  `packages/operation` models the optional-peer subpath pattern (valid
  observation, but the dependency shape is bound into the frozen Card 127
  candidate receipt — demotion deferred to the next distribution
  candidate); internal
  deps are bare `"0.1.0"` instead of `workspace:*`; ~15 workspace path deps
  lack `version =`; `rusqlite =0.31.0` is ~2 years behind with bundled-SQLite
  CVE bumps since.

### Doc drift (prose only; code backs all 137 card claims)

`README.md` ends four commits early ("Card 070 ready"), never records
g01.017-020 or g01 completion. `docs/contracts/contract-index.md` readiness
lines are stale. `docs/roadmaps/g01/batch-cards/README.md` holds Card 074
under Ready and a divergent next-task pointer competing with the generation
index. `CHANGELOG.md` counts 17/36 against actual 18/38 and has no fork-tree
entry. Contract 004 `Updated:` header missed `f2a78690`.
`docs/reference/api-surface.md` tables are accurate but predate three
public-surface commits.

### Consumer exposure

Eight relative-path consumers (nucleus deepest with an enforced
crate/package boundary verifier; figmatic vite-aliases into package source
files). Remediation is internal-only if: no `packages/*/src/` file moves, no
crate add/remove, and the `notifications/operation` and
`tauri-transfer/surface-transfer` feature names stay fixed. Async command
signatures do not change the invoke wire surface.

## Decision

Compile g02 as a remediation generation opening with six milestones: layout
sizing integrity, window lifecycle correctness, transfer session
truthfulness, host-thread and storage coordination, an injectable diagnostics
seam, and QA/docs alignment. All work executes under existing contracts.
