---
title: Agent control child-webview targeting worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-08-20
updated: 2026-08-20
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260820-163602-agent-control-webview-targeting-worker.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr]
---

## What This Thread Was Doing

Figmatic's preview island became screenshot-visible with g02.034, but
agents still cannot click or marquee inside it — semantic and input
tools were deliberately UI-webview-only. Figmatic triaged the gap
(`figmatic:docs/triage/20260820-162239-agent-control-preview-input-gap.md`)
and the operator made the product call: agents driving the preview is
the feature, not a nice-to-have. Contract 022 is already amended
(2026-08-20): an application may opt in named child-webview labels at
mount, opted-in children are full semantic targets with webview-scoped
refs, and the default stays closed.

Your run is milestone g02.035, Cards 239 and 240: make the amended
contract mechanism, prove it packaged, and hand Figmatic its opt-in
steps.

This file is your complete brief; the contract amendment and both cards
carry the design decisions.

## Why It Matters

The operator is currently the input device for preview acceptance —
driving clicks by hand because MCP stops at the island edge. That is
the exact failure this whole surface exists to end, one webview short.
The security boundary that kept it closed is preserved by shape, not
dropped: opt-in per label, default closed, because `evaluate` and
synthetic input inside a child execute in whatever content it hosts.

## Current State

Here is the state you're inheriting:

- **Repository:** `git@github.com:inflatable-cookie/longhorn.git`
- **Planning branch:** `main`
- **Planning base commit:** `48d0ec62d1dfeed15bbb88b6f2be8c17ea88ecb3`
- **Pushed main verification:** local `HEAD` equalled `origin/main` at
  that commit when this handoff was written.
- **Planning checkout:** clean apart from this handoff file.
- **Worker mode:** implementation worker dispatched by the
  orchestrator; this handoff activates the worker-only worktree
  preflight.
- **Planning artifacts included at the base:** contract 022 amended
  (Tool Surface opt-in rule + Boundaries update), milestone g02.035,
  Cards 239-240, and every prior agent-control lane merged through
  `e135d725`.
- **Worker branch:** `worker/239-agent-control-webview-targeting` — a
  label; the cards are 239 and 240.
- **Worker worktree:** launcher-provided when available; otherwise a
  unique worktree under `/Users/tom/Dev/worktrees` (operator-confirmed
  container).
- **Worktree creation command (manual fallback only):**
  `git worktree add /Users/tom/Dev/worktrees/longhorn-239 -b worker/239-agent-control-webview-targeting origin/main`
- **Worker worktree policy:** first use the clean, dedicated,
  non-`main` registered worktree supplied by the launcher, even if its
  generated path or branch differs from these placeholders. Record the
  actual path/branch and never create a second worktree for that
  reason. Only if the current context is `main`, dirty, unregistered,
  or otherwise unusable, use the named command above.
  `.agents.local.env` still does not exist (PAPERCUTS records why); the
  container above is the operator's standing answer. Never use `/tmp`,
  `TMPDIR`, or a guessed path.
- **Active spec lane:** none — the amended contract 022 is the
  authority; Cards 239-240 execute it.
- **Roadmap milestone:** g02.035
  (`docs/roadmaps/g02/035-agent-control-child-webview-targeting.md`)
- **Ready cards, in order:**
  `docs/roadmaps/g02/batch-cards/239-agent-control-webview-targeting.md`,
  `docs/roadmaps/g02/batch-cards/240-agent-control-child-drive-proof.md`
- **Allowed runway:** Cards 239-240, one PR.
- **Remaining card budget:** two cards; then stop and report.
- **Dispatch topology:** serial — 240 proves 239 packaged.
- **Parallel safety check:** not applicable; one lane.
- **Canonical refs:** `AGENTS.md`; contract 022 as amended (the opt-in
  rule is the spec — read its exact wording); the Figmatic triage note;
  contracts 001, 003 (no consumer repo edits), 006, 012 (wire
  additivity), 013; the Card 232/233/238 closeouts for the shim,
  marshalling, and per-webview mechanics you extend.
- **Expected seam-touches, pre-authorized:** `longhorn-agent-control`
  vocabulary and `server/args.rs`/`server/mcp.rs` (the additive
  `webview` field and typed refusals — this is the sanctioned core
  change this lane exists for), the plugin
  (`handler.rs`/`mount.rs`/`capture.rs`), the shim in
  `packages/longhorn/src/agent-control/` plus its committed bundle
  (keep the byte-lock fixture green), `examples/agent-control-proof`,
  `skills/agent-control/SKILL.md`, the composition guide, contract
  022's Required Evidence section. Anything else — new crates, effigy
  wiring, discovery schema, token/auth — is report-first through the
  operator.
- **Model capability profile:** capable coding model; the ref-scoping
  design and wire additivity deserve full care — two clients on two
  webviews of one window must never cross.
- **Tool/runtime restrictions:** macOS host; Card 240's packaged drive
  runs on the operator's display, no focus or pointer theft.
- **Required validation:** `effigy qa`; the release-absence scan both
  directions; the drift lock; the packaged drive and interleave.
- **PR base/head:** `main` ←
  `worker/239-agent-control-webview-targeting` (or the
  launcher-provided branch you recorded).
- **PR URL:** pending
- **Review state:** awaiting-review once opened
- **Merge authorisation:** operator only; never merge.

## Boundaries

Please keep this run inside the named runway:

- **In scope:** the opt-in config, `webview` addressing on
  semantic/input tools, webview-scoped refs, typed refusals, fixtures,
  the packaged drive and interleave proof, and the truthfulness pass
  across skill/guide/contract Required Evidence.
- **Out of scope:** consumer repos (Figmatic's opt-in is Figmatic's
  card — your closeout hands them steps, not commits); runtime
  mutation of the opt-in set; any default-open behavior; native
  (non-webview) surfaces and the provider seam; `screenshot`,
  `command`, discovery, auth (all unchanged).
- **Wire additivity is a hard rule** (contract 012): a request without
  `webview` must mean exactly what it means today; existing fixtures
  pass unmodified; old clients keep working.
- **Ref honesty is a hard rule**: cross-webview resolution fails as
  `UnresolvedRef` — never a wrong-element hit, and never a server-side
  ref table that outlives the DOM (contract 022's stateless posture; a
  stop condition if it can't be met).
- Shared checkout discipline: stage by explicit path, never
  `git add -A`, never stash.
- Work only in the selected clean worker worktree. Never edit the
  orchestrator's planning checkout.
- Do not merge the PR. Merge remains a separate operator-authorised
  action.

## Important Context

- **The plumbing is closer than the cards read cold:** the shim is
  already injected into every webview (Card 230's mount loop plus the
  initialization-script plugin — verify children attached *after*
  mount are covered, and fix in scope if not); the evaluate bridge
  already takes a per-webview handle (Cards 231/238); the handler
  already walks `Window` + `webviews()` (`c1482daf`). Card 239 is
  mostly addressing, scoping, and refusals — not new machinery.
- **Ref scoping is the one real design choice.** Refs today are
  `data-longhorn-agent-ref` attributes from a per-document counter, so
  two webviews produce colliding ref strings. The contract fixes the
  observable rule (a ref never resolves outside its webview); the
  card leaves the mechanism to you (per-webview prefixes in the shim
  vs. resolution keyed by the request's `webview` target) — decide,
  record why in the closeout, and fixture the collision case
  explicitly: same ref string, two webviews, no cross-hit.
- **The marquee question is the honest edge.** Figmatic's need is
  marquee selection in the preview. Untrusted `drag` dispatches DOM
  events; whether the island's selection logic responds to untrusted
  mousedown/mousemove/mouseup sequences is app-dependent. Card 240's
  proof app must include a drag-driven selection the untrusted
  sequence provably drives — and if untrusted drag *cannot* express
  some part of the motion, that limit is recorded precisely (stop
  condition), because Figmatic will hit whatever you overclaim.
- **Security text placement:** the guide's opt-in section carries the
  rule that opting in asserts the content is the app's own to drive,
  with `longhorn-browser`-hosted third-party content as the named
  counterexample. The contract already says it; the guide is where
  consumers will actually read it.
- **Report-first note:** the core-crate vocabulary change is
  pre-authorized here — that's the lesson from earlier lanes applied,
  scoped in rather than discovered mid-run. The rule stays strict for
  everything outside the seam list.
- **Report after:** (1) Card 239 vocabulary + scoping + refusals green
  under the existing suites with byte-compat proved; (2) Card 240's
  packaged drive and interleave evidence; then the PR.
- **Report to:** the operator, who relays progress to the
  orchestrator.

## Suggested Next Move

This handoff explicitly activates worker mode. Before broad repository
reads, run the quick startup worktree-safety preflight in
`## Completion Protocol`. If the current context is a clean, dedicated,
non-`main` registered worktree, it's the launcher-provided one: use it,
record its actual path/branch, and move on. Otherwise use the named
fallback command above.

Once the worktree checks out, read contract 022's amended Tool Surface
wording first — it is the spec — then both cards, the Figmatic triage
note, and the shim/handler/args sources. Start with the ref-scoping
design and its collision fixture; it's the piece everything else leans
on. When you reach a natural pause, tell the operator what changed,
what you actually ran, and what remains.

## Completion Protocol

### Before you start

1. Read this handoff. Its `worker_mode: implementation` and
   `dispatch_authority: orchestrator` metadata activate worker mode.
   Then run one quick read-only probe before broad reads:
   `git rev-parse --show-toplevel`, `git branch --show-current`,
   `git status --porcelain`, `git worktree list --porcelain`.
2. If the current root is a registered worktree with empty status on a
   non-`main` branch, accept it as the launcher-provided worktree.
   Record its actual root/branch; do not create another because it
   differs from the placeholders here.
3. Only if the current context is `main`, dirty, unregistered, or
   unusable: run the named worktree command above (container
   `/Users/tom/Dev/worktrees`, operator-confirmed). Never clean,
   reset, or stash a dirty checkout. If the launcher supplied a dirty
   or `main` worktree, stop and report it.
4. From the selected worktree, after `git fetch origin`, confirm
   `git rev-parse HEAD` equals `git rev-parse origin/main`, confirm
   `git merge-base --is-ancestor 48d0ec62d1dfeed15bbb88b6f2be8c17ea88ecb3 HEAD`
   succeeds, and confirm this handoff file exists in `HEAD`.
5. Read the amended contract 022, both cards, milestone g02.035,
   `AGENTS.md`, and the Card 232/233/238 closeouts.
6. Run the repo's cheap orientation (`effigy tasks`, `effigy doctor`)
   and record what you actually ran.

### While you work

- Execute Card 239 then Card 240. Keep commits aligned with meaningful
  chunks.
- After each meaningful chunk, report through the operator: changed
  files, validation actually run, remaining cards, new risks,
  blockers.
- Stop and say so if: ref scoping needs state that outlives the DOM;
  the wire change cannot stay additive; untrusted drag cannot express
  the selection motion; the shim cannot reach late-attached children
  within scope; or `effigy qa` fails in a way that changes the plan.
- Edits outside the pre-authorized seam list: report first, then land
  with evidence.
- Do not quietly turn an open question into new architecture.

### When the assigned runway is complete

1. Final validation: `effigy qa`; the release-absence scan both ways;
   the drift lock; the packaged drive, refusal leg, and interleave.
2. Update both cards' acceptance boxes and closeouts (ref-scoping
   design and why, evidence paths, resolved versions, actual
   worktree/branch, the landed revision and Figmatic opt-in steps),
   tick the milestone, apply the skill/guide/contract truthfulness
   pass, and write a dated log with an index line.
3. Push the worker branch.
4. Open a PR against `main`. The base above is the planning commit
   before this handoff existed, not a self-referential hash.
5. In the PR body, link the milestone, cards, the contract amendment,
   the triage note, changed surfaces, evidence, validation, and
   unresolved items.
6. Report the PR URL and evidence to the operator. Do not merge.

### Review and merge path

The orchestrator reviews the PR against the amended contract 022, both
cards, and the diff, independently of your narrative — including
running the collision and byte-compat fixtures and reading the packaged
evidence directly. Orchestrator and worker share a GitHub identity, so
the verdict lands as a PR comment — that comment is the canonical
review record. If changes are requested, make only those changes on
this branch and push again. The operator alone authorises merge.

- **Closeout refs:** Cards 239-240;
  `docs/roadmaps/g02/035-agent-control-child-webview-targeting.md`;
  `docs/roadmaps/g02/README.md`; `docs/roadmaps/generation-index.md`
  (Next Task pointer is updated by the orchestrator at merge, not by
  you).

### Handoff closeout

Before calling the runway complete, leave the cards, milestone,
contract, and evidence state honest. The marquee edge is where
overclaiming would hurt most — Figmatic automates against exactly what
your closeout says untrusted drag can do, so say precisely that and no
more.
