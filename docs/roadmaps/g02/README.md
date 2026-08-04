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
| [g02.008](008-distribution-candidate-v2.md) | ready | candidate receipt v2 over the refreshed graph |

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

## Current Checkpoint

Research memo 018 promotes the post-g01 workspace audit. All findings sit
inside contracts 004, 010, 011, 012, 014, and 017; no new contract gates
execution. Cards 138-147 compile the six-milestone remediation runway.
The remediation milestones and the dependency refresh sweep (Cards
138-148) are complete. Card 149 (distribution candidate v2) is ready and
closes the Tier A lane.

## Candidate Runway

Deferred candidates compiled from g02 evidence and the
[planning-gaps register](../../architecture/system-inventory.md#planning-gaps),
tiered by readiness. None auto-starts; the operator picks.

### Tier A — executable under existing contracts

| Candidate | Evidence | Shape |
| --- | --- | --- |
| Dependency refresh sweep | 37 transitive crates behind within compatible ranges; four held-back pins under frozen evidence: rusqlite 0.31→0.40 (SQLite CVE bumps), zip 5.1→8.6 (backup-archive determinism), ts-rs 11→12 (full bindings regen), sha2 0.10→0.11 | one card, ordered before candidate v2: compatible `cargo update`, then each held-back crate with its conformance suite (SQLite adapter proofs, archive-determinism regression, bindings and fixture diff) |
| Distribution candidate v2 | Card 127 receipt frozen at 17/36; repo is 18/38; bridge optional-peer demotion deferred here (Card 146) | one card, after the sweep: new private candidate receipt superseding Card 127 over the refreshed graph, bridge peer demotion, refreshed counts, commit-pinned proof re-frozen |
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

## Continuation

The [generation index](../generation-index.md) owns the only live next-task
pointer.
