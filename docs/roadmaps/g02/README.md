# g02 Workspace Integrity Remediation

## Generation Runway

| Milestone | State | Outcome |
| --- | --- | --- |
| [g02.001](001-layout-sizing-integrity.md) | complete | serde-proof layout ratio and sizing invariants |
| [g02.002](002-window-lifecycle-correctness.md) | complete | non-blocking event loop, coherent retag, safe install, timer wakes |
| [g02.003](003-transfer-session-truthfulness.md) | complete | truthful consumed-session aborts and race-free client bindings |
| [g02.004](004-host-thread-and-storage-coordination.md) | complete | off-main-thread storage commands and self-healing restore loads |
| [g02.005](005-injectable-diagnostics-seam.md) | complete | evidence for every best-effort failure swallow |
| [g02.006](006-qa-and-docs-alignment.md) | complete | resolving QA selectors, package hygiene, truthful front doors |
| [g02.007](007-dependency-refresh-sweep.md) | complete | current dependency graph with conformance-backed pin decisions |
| [g02.008](008-distribution-candidate-v2.md) | operator-held | candidate receipt v2 over the refreshed graph |
| [g02.009](009-application-update-and-release-channels.md) | in progress | in-app update: channels, client-side rollout, source adapters, restart safety |
| [g02.010](010-licensing-entitlement-and-activation.md) | ready | licensing: opaque entitlements, use/update windows, activation adapters |
| [g02.011](011-ipc-boundary-validation.md) | ready | IPC boundary validation derived from the Rust authority |
| [g02.012](012-dual-backend-host-positioning.md) | ready | two first-class hosts: Tauri and GPUI, one host-agnostic core |
| [g02.013](013-typescript-package-graph.md) | complete | eighteen TypeScript packages to three, grouped by peer requirement |

The runway is open-ended: g02 continues past remediation into whatever shared
gap consumer adoption characterizes next. Deferred candidates in the
[system inventory](../../architecture/system-inventory.md#planning-gaps)
remain uncommitted.

## Dependency Shape

```text
memo 018 workspace audit
 ├─ 001 layout sizing integrity
 ├─ 002 window lifecycle correctness ─┐
 ├─ 003 transfer session truthfulness ├─ 005 diagnostics seam
 ├─ 004 host thread and storage      ─┘
 └─ 006 QA and docs alignment
```

001-004 and 006 are independent. 005 lands after 002-004 settle the swallow
sites it instruments.

```text
memo 019 application update
 └─ 009 application update and release channels
     ├─ 150 cross-channel store proof     (independent)
     └─ 151 update policy ─┬─ 152 source adapters
                           └─ 153 restart interlock ─ 154 client surface
```

```text
memo 020 licensing
 └─ 010 licensing, entitlement, and activation
     └─ 155 licence core ─┬─ 156 activation adapters
                          └─ 157 host and storage ─ 158 client surface
```

```text
P2-10 audit finding -> Card 160 inventory
 └─ 011 IPC boundary validation
```

```text
memo 021 dual-backend positioning
 └─ 012 dual-backend host positioning
     ├─ 161 positioning and leak fixes   (independent)
     ├─ 162 native update execution      (independent)
     └─ 163 GPUI host adapter            (validates 161's contract)
```

009, 010 and 011 are independent of 001-008 and of the poodle release
blocking the v0.1.0 tag. 009 and 010 meet at the update window: the updater
asks the licence whether a release may be taken. 011 touches neither.

## Current Checkpoint

Research memo 018 promotes the post-g01 workspace audit. All findings sit
inside contracts 004, 010, 011, 012, 014, and 017; no new contract gates
execution. Cards 138-147 compile the six-milestone remediation runway.
The remediation milestones and the dependency refresh sweep (Cards
138-148) are complete. Card 149 part 1 (bridge optional peer, candidate
verifier, diagnostics guide) is done; receipt generation is operator-held
on nucleus manifest quiescence.

Research memo 019 opens the generation past remediation: contract 018
compiles the in-app update boundary, and Cards 150-154 (g02.009) execute it.
Cards 150-152 are complete. Card 153 has its mechanism findings recorded and
its quiescence contract landed; its host wiring waits on a packaged proof
application.

Research memo 020 compiles contract 019 over licensing. Cards 155-158
(g02.010) are ready.

Research memo 021 repositions the framework: Tauri and GPUI become two
first-class, permanent hosts. Measurement found the separation largely
already exists — 12.5% of the Rust is Tauri-bound, with exactly one
pure-to-host dependency edge and one webview-shaped concept in core.
Contract 020 compiles the host-adapter boundary, contract 018 is amended so
update execution is host-dependent, and the register is tiered into core,
host, and webview edge. Cards 161-163 (g02.012) execute it.

g02.011 opens from measurement rather than a memo. The audit's P2-10 finding
named 5,330 lines; Card 160's inventory turned that into a coverage table
showing nine of thirteen packages never validate keys at the IPC boundary,
and 29 hardcoded bounds with no link to the 55 Rust constants defining them.
All three new milestones are independent of the poodle release currently
blocking the v0.1.0 tag.

## Candidate Runway

Deferred candidates compiled from g02 evidence and the
[planning-gaps register](../../architecture/system-inventory.md#planning-gaps),
tiered by readiness. None auto-starts; the operator picks.

### Tier A — executable under existing contracts

| Candidate | Evidence | Shape |
| --- | --- | --- |
| Dependency refresh sweep | 37 transitive crates behind within compatible ranges; four held-back pins under frozen evidence: rusqlite 0.31→0.40 (SQLite CVE bumps), zip 5.1→8.6 (backup-archive determinism), ts-rs 11→12 (full bindings regen), sha2 0.10→0.11 | one card, ordered before candidate v2: compatible `cargo update`, then each held-back crate with its conformance suite (SQLite adapter proofs, archive-determinism regression, bindings and fixture diff) |
| Distribution candidate v2 | Card 127 receipt frozen at 17/36; repo is 18/41; bridge optional-peer demotion deferred here (Card 146) | one card, after the sweep: new private candidate receipt superseding Card 127 over the refreshed graph, bridge peer demotion, refreshed counts, commit-pinned proof re-frozen |
| Diagnostics adoption | Card 145 seam has no consumer guide | one small card: composition-guide section plus one consumer-shape example |

### Tier B — needs new evidence or a contract extension first

- live native scale-transition proof for child-view and backing-surface
  (unmet since g01.018; needs a multi-monitor host or simulation authority)
- Windows/Linux child-view proof and non-macOS display/transfer evidence
  (needs platform hosts)
- native OS notification delivery (contract 016 extension)
- macros, extended input triggers, native accelerators, synchronized keymaps
  (contract 006 extension; Loophole Echo edge recorded)
- cross-document panel transaction and copy-transfer authority (contract 011
  extension)

### Tier C — operator product decision before research

- server-synchronized configuration and remote conflict semantics
- production backend transport and endpoint-security policy
- durable offline mutation policy
- secure credential-store provider selection
- package-registry naming and publication (explicitly deferred)

## Consumer Guardrails

Remediation stays internal to Longhorn. No `packages/*/src/` file moves
(figmatic vite aliases), no crate or package add/remove (nucleus boundary
verifier), and the `notifications/operation` and
`tauri-transfer/surface-transfer` feature names stay fixed. Async command
migration must not change the invoke wire surface.

**These guardrails scope to remediation.** g02.009 and g02.010 are new
capability work and add crates and packages deliberately. The additions are
inert until composed; consumers pick them up when they adopt the feature.

## Continuation

The [generation index](../generation-index.md) owns the only live next-task
pointer.
