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
| [g02.011](011-ipc-boundary-validation.md) | complete | IPC boundary validation derived from the Rust authority |
| [g02.012](012-dual-backend-host-positioning.md) | complete | two first-class hosts: Tauri and GPUI, one host-agnostic core |
| [g02.013](013-typescript-package-graph.md) | complete | eighteen TypeScript packages to three, grouped by peer requirement |
| [g02.014](014-first-publication.md) | ready | Poodle and Longhorn on public npm; consumers on versions; v0.1.0 tagged |
| [g02.015](015-gpui-composition-and-live-evidence.md) | complete | one guide assembles a GPUI application; contract 020's last two ceilings proved live |
| [g02.016](016-fork-history-field-corrections.md) | complete | five fork-history items from Loophole's field use; three delete live workarounds |
| [g02.017](017-fork-retention-and-deletion.md) | in progress | an operator can delete a fork; retention can prune, which today it cannot |
| [g02.020](020-no-design-in-the-authority.md) | ready | the authority binds; the design system designs. No CSS in longhorn-poodle-svelte |
| [g02.019](019-the-origin-position.md) | in progress | the state the operator started from is a position they can name and return to |
| [g02.018](018-tagged-union-boundary-validation.md) | complete | the 181 tagged unions validate per variant, from the Rust enums |
| [g02.021](021-update-supply-chain-hardening.md) | complete | the update install path's stated properties are true |
| [g02.022](022-bridge-session-and-event-lifecycle.md) | complete | events respect read authority; sessions end with their window |
| [g02.023](023-credential-and-activation-hardening.md) | complete | the credential path matches the config-age discipline |
| [g02.024](024-coverage-restoration.md) | complete | the test surface matches the code surface again |
| [g02.025](025-docs-spine-reconciliation.md) | complete | the handwritten spine describes the system that exists |
| [g02.026](026-automation-and-gate-convergence.md) | in progress — 219/220 landed; 218 held on Poodle v0.2.0 | gates mean what they claim; one declaration per fact |
| [g02.027](027-structural-consolidation-and-dependency-sweep.md) | complete | rules stated once are implemented once |
| [g02.028](028-consumer-scoped-credential-slots.md) | complete | one shared store admits isolated consumer-owned scope without product policy |
| [g02.029](029-agent-control-spike.md) | complete | memo 024's two runtime unknowns answered with recorded prototype evidence |
| [g02.030](030-agent-control-core.md) | complete | `longhorn-agent-control`: stateless MCP core, discovery, token, provider seam |
| [g02.031](031-agent-control-tauri-host.md) | complete | Tauri plugin: dev-only server mount, unfocused capture, release-absence proof |
| [g02.032](032-agent-control-semantic-surface.md) | done 2026-08-19 | semantic snapshot and input tools; packaged unfocused end-to-end proof |
| [g02.033](033-agent-control-adoption-kit.md) | done 2026-08-19 | adoption kit: composition guide, canonical drift-locked skill, one-command install, dogfood proof |
| [g02.034](034-agent-control-window-composition.md) | ready | screenshot composes child webviews: whole-window truth for multiwebview apps |

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
contract 020 ceilings + memo 022
 └─ 015 GPUI composition and live evidence
     ├─ 172 build cadence            (blocks 174-176)
     ├─ 173 composition guide        (independent)
     └─ 174 worked example ─┬─ 175 live cross-window drag
                            └─ 176 live teardown under load
```

```text
memo 021 dual-backend positioning
 └─ 012 dual-backend host positioning
     ├─ 161 positioning and leak fixes   (independent)
     ├─ 162 native update execution      (independent)
     └─ 163 GPUI host adapter            (validates 161's contract)
```

```text
memo 023 full-repo release audit (cards 200-223)
 ├─ 021 update supply-chain hardening      (200-203; 203 last, amends 018)
 ├─ 022 bridge session and event lifecycle (204 decision → 205, 206)
 ├─ 023 credential and activation hardening(207-209 independent, 210 after 208)
 ├─ 024 coverage restoration               (211-214 independent)
 ├─ 025 docs spine reconciliation          (215 → 216, 217)
 ├─ 026 automation and gate convergence    (218 held on Poodle v0.2.0;
 │                                            219, 220 independent)
 └─ 027 consolidation and dependency sweep (221, 222 independent; 223 after 222)
```

```text
memo 024 agent app control (contract 022 draft)
 └─ 029 spike (card 227) ── promotion gate (memo 024 + contract 022)
     └─ 030 core (228, 229) ─ 031 tauri host (230, 231)
         └─ 032 semantic surface and proof (232-234)
```

The build runway is complete (PRs 2-5, all 2026-08-19): contract 022
active with its Required Evidence cited or explicitly narrowed, two
crates and the TS shim merged, release-absence proved by scan both
directions, and the packaged end-to-end proof driven by a real MCP
client while the app never held OS focus.

```text
g02.029-032 complete
 └─ 033 adoption kit (235 guide+skill → 236 install+drift → 237 dogfood)
     └─ per-app adoption cards, run from the five consumer repos
```

g02.033 is complete (PR 6): the composition guide, the drift-locked
skill and finder, one-command install proved against a scratch repo,
and the dogfood pass — which found and fixed a real listen-cursor bug,
exactly its job. The whole agent-control arc, memo to adoption kit, ran
2026-08-19 in six PRs. What remains is per-app adoption from the five
consumer repos' own roadmaps (contract 003); the stdio proxy client
sits in the candidate runway.

Adoption evidence reopened one seam: Figmatic's preview child webview
is black in `screenshot` output — the capture mechanism (UI-webview
`takeSnapshot`) cannot see sibling webview pixels, so contract 022's
screenshot claim currently overclaims. The Figmatic handoff
(`docs/handoffs/20260820-142528-figmatic-child-webview-screenshot-composition.md`)
characterizes it; g02.034 (Card 238) makes the mechanism match the
claim or narrows it honestly.

021-027 are independent of each other and of 014-020; all execute inside
existing contracts. The audit found the engineering core sound — these
milestones harden the evidence envelope and the surfaces that grew faster
than their proof.

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
Contract 020 compiles the host-adapter boundary and the register is tiered
into core, host, and webview edge. **g02.012 is complete.**

The second backend exists, and contract 020 is amended from what building it
found — thirteen bends, eight GPUI's shape and four Tauri assumptions the
contract had absorbed, plus one found only by attaching a second monitor.
The contract carries a divergence register and says in its own Evidence
section that it is still not complete, because no backend has proved
multi-window placement, cross-window transfer, or lifecycle teardown under
load.

Contract 018 was amended twice. On 2026-08-08 update execution became
host-dependent; on 2026-08-09 it became **host-independent**, after Card 162
established that the Tauri updater plugin cannot implement the shared
conformance suite at all — its verification is welded to its own downloader,
and its `install` takes caller-supplied bytes unverified. One installer now
serves both hosts, proved against a real packaged application bundle.

Research memo 023 compiles the full-repo release audit of 2026-08-14: nine
lanes over every crate, package, script, workflow, and the docs spine. The
engineering core held (type-forced verification, per-caller authorization,
zero reachable panic on untrusted input); the findings concentrate in the
evidence envelope — extraction that is not bounded against link entries, a
docs spine still specifying the Card 179 deletion, deleted-not-ported layout
test suites, and gates that drift by hand-transcription. Cards 200-223
compile the remediation into milestones g02.021-027, all inside existing
contracts. Card 218's closure is operator-held on the Poodle v0.2.0 release
that unblocks g02.014.

Research memo 024 characterizes the agent app-control gap: agents testing
consumer apps steal OS focus and the pointer because no remote-control
protocol reaches macOS Tauri. Card 227's spike (PR 2) answered both
runtime unknowns positive — rmcp mounts statelessly in-process and Claude
Code negotiates 2026-07-28 session-free; `takeSnapshot` is fresh
unfocused, occluded, and minimized — plus one new finding folded into the
contract: WKWebView stops rAF while not key, so waiting is DOM-relative.
Memo 024 and contract 022 are promoted; g02.030 (Cards 228-229) is ready.

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
| Agent-control stdio proxy client | g02.033 skill connects dynamically (ephemeral port + token); a static `.mcp.json` needs a command-based proxy | one card: `longhorn-agent-control-client` binary — discovers the live instance, proxies stdio↔streamable-HTTP; distribution decision (crate bin vs npm) inside the card |

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
- ~~package-registry naming and publication~~ — promoted to g02.014 on
  2026-08-09 when the `@inflatable-cookie` scope was claimed

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
