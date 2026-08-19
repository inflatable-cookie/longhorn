---
title: Agent control semantic surface worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-08-19
updated: 2026-08-19
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260819-194148-agent-control-semantic-surface-worker.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr]
---

## What This Thread Was Doing

The agent app-control runway is three lanes in: spike proved (Card 227,
PR 2), core crate merged (Cards 228-229, PR 3), Tauri host and packaged
capture merged (Cards 230-231, PR 4). Your run is the lane that closes the
feature: milestone g02.032, Cards 232, 233, and 234 — the in-page semantic
shim, the tools that make the plugin's remaining `Unsupported` answers
real, and the end-to-end proof that an agent drives a packaged app while
it never holds OS focus.

This file is your complete brief; you don't need the planning transcript.

## Why It Matters

Everything so far is plumbing an agent could reach but not usefully hold:
`evaluate`, `screenshot`, `command`, window ops. This lane adds the
surface agents actually work with — semantic snapshots with refs, clicks
and typing, DOM-relative waiting, and pushed console/error/navigation
events. Card 234 then walks contract 022's Required Evidence item by item
and closes it. When this lane merges, the reason the feature exists —
agents stealing the operator's focus and pointer to test apps — is
structurally over, and adoption moves to the five consumer apps.

## Current State

Here is the state you're inheriting:

- **Repository:** `git@github.com:inflatable-cookie/longhorn.git`
- **Planning branch:** `main`
- **Planning base commit:** `bac9a64675e51b8fa2ad2d289ffc65d0df3464af`
- **Pushed main verification:** local `HEAD` equalled `origin/main` at
  that commit when this handoff was written.
- **Planning checkout:** clean apart from this handoff file.
- **Worker mode:** implementation worker dispatched by the orchestrator;
  this handoff activates the worker-only worktree preflight.
- **Planning artifacts included at the base:** contract 022 (active),
  merged `longhorn-agent-control` and `longhorn-tauri-agent-control`,
  `examples/agent-control-proof` with the Card 231 matrix driver and
  evidence, milestone g02.032 (ready), Cards 232-234.
- **Worker branch:** `worker/232-agent-control-semantic-surface` — the
  branch number is a label; the cards are 232, 233, and 234.
- **Worker worktree:** launcher-provided when available; otherwise a
  unique worktree under `/Users/tom/Dev/worktrees` (operator-confirmed
  container).
- **Worktree creation command (manual fallback only):**
  `git worktree add /Users/tom/Dev/worktrees/longhorn-232 -b worker/232-agent-control-semantic-surface origin/main`
- **Worker worktree policy:** first use the clean, dedicated, non-`main`
  registered worktree supplied by the launcher, even if its generated
  path or branch differs from these placeholders. Record the actual
  path/branch and never create a second worktree for that reason. Only if
  the current context is `main`, dirty, unregistered, or otherwise
  unusable, use the named command above. `.agents.local.env` still does
  not exist (PAPERCUTS records why); the container above is the
  operator's standing answer. Never use `/tmp`, `TMPDIR`, or a guessed
  path.
- **Active spec lane:** none open — contract 022 and memo 024 are
  promoted background. Card 234 amends contract 022 only through its
  report-first rule.
- **Roadmap milestone:** g02.032
  (`docs/roadmaps/g02/032-agent-control-semantic-surface.md`)
- **Ready cards, in order:**
  `docs/roadmaps/g02/batch-cards/232-agent-control-webview-shim.md`,
  `docs/roadmaps/g02/batch-cards/233-agent-control-semantic-tools.md`,
  `docs/roadmaps/g02/batch-cards/234-agent-control-end-to-end-proof.md`
- **Allowed runway:** Cards 232-234, one PR.
- **Remaining card budget:** three cards; then stop and report.
- **Dispatch topology:** serial — 233 wires 232's shim; 234 proves 233's
  tools.
- **Parallel safety check:** not applicable; one lane.
- **Canonical refs:** `AGENTS.md`; contract 022
  (`docs/contracts/022-agent-app-control.md`) — its Required Evidence
  section is Card 234's checklist; contracts 001, 006, 010, 013
  (TS packaging), 020 (per-backend evidence discipline); memo 024; the
  Card 229/230/231 closeouts for the seams you build on.
- **Model capability profile:** capable coding model; the ref-stability
  and event-buffering semantics deserve full care — they're what two
  concurrent agents lean on.
- **Tool/runtime restrictions:** macOS host; network for crates.io/npm.
  Cards 233-234 build and run the packaged proof app on the operator's
  display.
- **Required validation:** `effigy qa`; the release-absence scan both
  directions (shim asset and injection code must stay gated); the
  packaged end-to-end driver and two-client interleave; `effigy doctor`
  for orientation.
- **PR base/head:** `main` ← `worker/232-agent-control-semantic-surface`
  (or the launcher-provided branch you recorded).
- **PR URL:** pending
- **Review state:** awaiting-review once opened
- **Merge authorisation:** operator only; never merge.

## Boundaries

Please keep this run inside the named runway:

- **In scope:** the shim in the `longhorn` TS package (Card 232), tool
  wiring and `subscriptions/listen` events in the plugin (Card 233), the
  extended proof app, end-to-end evidence, and the contract 022 Required
  Evidence closeout (Card 234) — plus the card/milestone/log surfaces the
  cards name.
- **Out of scope:** consumer repos; native-surface providers (the seam
  stays empty); trusted OS input in any form; new tools beyond the
  contract 022 surface; GPUI anything.
- **Report-first rule (this one is load-bearing):** any change to the
  core crate's public surface, the plugin's public surface, or contract
  022 gets reported through the operator *before* the edit — even an
  obviously correct fix. PR 4 landed a correct core fix without the
  mid-run report; the review accepted it with exactly this note. Card
  234's contract amendments follow the same rule: report, then land the
  edit with its evidence.
- Do not invent architecture, widen the roadmap, or settle an unresolved
  product/API/persistence/security decision.
- Shared checkout discipline: stage by explicit path, never `git add -A`,
  never stash.
- Work only in the selected clean worker worktree. Never edit the
  orchestrator's planning checkout.
- Do not merge the PR. Merge remains a separate operator-authorised
  action.

## Important Context

- **Planning lineage:** memo 024 → contract 022 → PRs 2, 3, 4. The three
  lane logs under `docs/logs/2026-08/19-agent-control-*.md` are the short
  history; the g02 README dependency shape shows this as the last lane.
- **Why these cards are ready:** every seam this lane needs is merged and
  proved — the core vocabulary and server (PR 3), the evaluate bridge,
  window scope, injection point, and packaged-proof pattern (PR 4).
  Contract 022 is active and its `wait_for`/screenshot semantics already
  carry the spike's rAF/timer findings.
- **Decisions already made (don't relitigate):** refs are stamped into
  the live DOM and resolved there — no shim- or server-side registry
  that outlives the DOM; `wait_for` is DOM-relative, pacing host-side,
  and no time-only or rAF wait may exist anywhere in the path; synthetic
  input is untrusted and documented as such; event buffering is bounded,
  drop-oldest, with a surfaced drop counter; the shim ships from the
  `longhorn` package per contract 013 and is injected only behind `dev`.
- **Seams you build on (read their closeouts):** Card 231's capture
  bridge is the evaluate marshalling path; Card 230's mount owns
  injection and lifecycle (note its `RunEvent::Exit` finding); Card 229's
  `server/args.rs` shows the wire-args pattern and its closeout records
  the `name = "type"` lesson — assert the full wire vocabulary when you
  add tools.
- **The known unknown:** rmcp 3.1.3's support for `subscriptions/listen`
  on the revisions current clients negotiate is unverified — Card 233
  names it as a stop condition. Probe it early, before building the
  event pipeline on top; if rmcp can't carry it, stop and report with
  what it does support.
- **Packaged-run logistics:** the Card 231 driver pattern (build via
  `bunx @tauri-apps/cli build`, launch, read discovery, drive over MCP)
  is the donor for Card 234's driver. Runs need the operator's display;
  coordinate the moment through the operator. The two-client interleave
  and focus check (System Events `frontmost = false` throughout) are
  evidence, not narration — commit them.
- **Workspace bootstrap:** expect the Card 229 closeout's `effigy qa`
  bootstrap path to apply (bun install, Poodle link, `POODLE_REPO`).
- **Report after:** (1) Card 232 shim fixtures green; (2) Card 233
  round-trip and listen fixtures green with the scan clean both ways;
  (3) Card 234's packaged evidence recorded; then the PR.
- **Report to:** the operator, who relays progress to the orchestrator.

## Suggested Next Move

This handoff explicitly activates worker mode. Before broad repository
reads, run the quick startup worktree-safety preflight in
`## Completion Protocol`. If the current context is a clean, dedicated,
non-`main` registered worktree, it's the launcher-provided one: use it,
record its actual path/branch, and move on. Otherwise use the named
fallback command above.

Once the worktree checks out, read `AGENTS.md`, contract 022, milestone
g02.032, all three cards, and the Card 229-231 closeouts. Two early moves
pay for themselves: probe rmcp's `subscriptions/listen` support before
building on it, and get the shim's ref/tree fixtures green before any
plugin wiring — the shim's semantics are what everything downstream
trusts. When you reach a natural pause, tell the operator what changed,
what you actually ran, and what remains.

## Completion Protocol

### Before you start

1. Read this handoff. Its `worker_mode: implementation` and
   `dispatch_authority: orchestrator` metadata activate worker mode. Then
   run one quick read-only probe before broad reads:
   `git rev-parse --show-toplevel`, `git branch --show-current`,
   `git status --porcelain`, `git worktree list --porcelain`.
2. If the current root is a registered worktree with empty status on a
   non-`main` branch, accept it as the launcher-provided worktree. Record
   its actual root/branch; do not create another because it differs from
   the placeholders here.
3. Only if the current context is `main`, dirty, unregistered, or
   unusable: run the named worktree command above (container
   `/Users/tom/Dev/worktrees`, operator-confirmed). Never clean, reset,
   or stash a dirty checkout. If the launcher supplied a dirty or `main`
   worktree, stop and report it.
4. From the selected worktree, after `git fetch origin`, confirm
   `git rev-parse HEAD` equals `git rev-parse origin/main`, confirm
   `git merge-base --is-ancestor bac9a64675e51b8fa2ad2d289ffc65d0df3464af HEAD`
   succeeds, and confirm this handoff file exists in `HEAD`.
5. Read all three cards, milestone g02.032, `AGENTS.md`, contract 022,
   and the Card 229-231 closeouts.
6. Run the repo's cheap orientation (`effigy tasks`, `effigy doctor`) and
   record what you actually ran.

### While you work

- Execute Cards 232, 233, 234 in order. Keep commits aligned with
  meaningful chunks.
- After each meaningful chunk, report through the operator: changed
  files, validation actually run, remaining cards, new risks, blockers.
- Stop and say so if: rmcp cannot carry `subscriptions/listen` for
  current clients; the semantic tree needs vocabulary the core cannot
  express; ref stability would need a registry outliving the DOM; the
  release-absence scan cannot stay clean with the shim gated; a contract
  amendment wants to widen rather than narrow; or `effigy qa` fails in a
  way that changes the plan.
- Any public-surface or contract edit: report first, then land it with
  its evidence.
- Do not quietly turn an open question into new architecture.

### When the assigned runway is complete

1. Run the required final validation: `effigy qa`, the release-absence
   scan both ways, the packaged end-to-end driver and two-client
   interleave; record results honestly.
2. Update all three cards' acceptance boxes and closeouts (resolved
   versions, actual worktree/branch, committed evidence paths), tick the
   milestone runway, and write the lane log under `docs/logs/2026-08/`
   with an index line in `docs/logs/README.md`.
3. Walk contract 022's Required Evidence section per Card 234 — cite or
   amend, nothing implied.
4. Push the worker branch.
5. Open a PR against `main`. The base above is the planning commit before
   this handoff existed, not a self-referential hash.
6. In the PR body, link the milestone, cards, contract 022, changed
   surfaces, evidence, validation, and unresolved items.
7. Report the PR URL and evidence to the operator. Do not merge.

### Review and merge path

The orchestrator reviews the PR against contract 022, the cards, and the
diff, independently of your narrative. Orchestrator and worker share a
GitHub identity, so the verdict lands as a PR comment — that comment is
the canonical review record. If changes are requested, make only those
changes on this branch and push again. The operator alone authorises
merge.

- **Closeout refs:** Cards 232-234;
  `docs/roadmaps/g02/032-agent-control-semantic-surface.md`;
  `docs/roadmaps/g02/README.md`; `docs/roadmaps/generation-index.md`
  (Next Task pointer is updated by the orchestrator at merge, not by
  you).

### Handoff closeout

Before calling the runway complete, leave the cards, milestone, contract,
and evidence state honest. This lane closes a contract's evidence
section — an implied claim there outlives the run that implied it, so if
anything is blocked or narrowed, record it and stop rather than rounding
up.
