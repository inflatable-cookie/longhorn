---
title: Longhorn Chatterbox refresh — 0.2.0 shipped; agent file-selection boundary is next
kind: northstar-handoff
handoff_mode: chatterbox-continuation
chatterbox_mode: conversational-planning
dispatch_authority: chatterbox
status: ready-to-launch
---

# Longhorn Chatterbox refresh

## What This Thread Was Doing

This thread ran Longhorn through its second release and then straight into a new
product question.

It picked up with `0.1.0` published and the runway converging on a second
release. Four lanes were planned and carried to completion: a staged update
protocol, an exclusive admission lease (both amending contract 018), a packaged
`agent-control` opt-in (amending contract 022 and withdrawing contract 023's
production MCP role), and an agent-control stdio carrier for harnesses that
cannot speak streamable HTTP. A zero-scope route-commit race in the Poodle
settings session surfaced during consumer acceptance and was fixed with a
monotonic route generation; the consumer's original diagnosis was wrong, and
naming the real root cause mattered.

The release then had to *earn* its way out. The operator held it until a real
consumer proved it: `effigy deps link` against the live checkout, then Desktop
acceptance on the linked candidate. That evidence landed, and `0.2.0` was
published — with one incident on the way, described under Important Context.

At the end of the thread the operator raised a genuinely new gap: when an app
opens a file picker, an agent driving it over MCP hits a wall and a human has to
step in. That is where this thread leaves off, and it is the most interesting
open question on the board.

## Why It Matters

Longhorn is the shared desktop substrate for five sibling apps: generic Rust,
TypeScript, Svelte, host adapters, and the cross-language contracts that bind
them. Agent control is its one packaged agency surface — the thing that lets an
agent actually drive an application instead of guessing at pixels.

A file picker is the first *common* flow that stops agent control dead. It is not
an edge case: four of the consumers call `@tauri-apps/plugin-dialog` roughly
forty-eight times between them, and one consumer has already recorded that a
lane cannot be proven live because of it. Every consumer acceptance lane that
touches "open a project", "import a file", or "choose a folder" inherits this
wall. Fixing it is what turns agent control from "can drive the UI" into "can
drive the work".

The release that just shipped matters for a different reason: it is the first
proof that Longhorn's release path survives a real consumer gate, including a
failure the local gates could not see.

## Current State

**Thread and workspace.** Source Chatterbox `18b7b953-373a-4599-8aed-a29ff2a726cf`,
in workspace `wks_edde3deb8d63dbd7` — `/Users/tom/Dev/projects/longhorn`,
`local` isolation, title "Longhorn release". `main` is clean and pushed. The
last substantive commit is `0cea8d3f`; this handoff and the file-selection
triage note are the only commits on top of it, so expect `main` to be one or two
ahead of that name.

**Queue preflight.** Clean. `node bin/northstar-transfer-origin.mjs preflight`
returned plan `f6b208af5507a10701f36151f92bced0e95881e24825b05b24faa6d3528e86c4`
with an **empty task set** — this Chatterbox has no unfinished tasks attached to
it, so there is nothing to transfer and no attention to inherit. The two live
Queue tasks that touch Longhorn's work (`7bf46d0e` = acowtancy `g05.179`,
`186267ae` = acowtancy `g05.180`) belong to the acowtancy/Farmyard project and
are routed there, not here.

**Released.** Longhorn `0.2.0` is published.

- Tag `v0.2.0` (annotated) → `df09c25a2fd1f6e8cc16c85961c81410f4dfc75e`.
- `@inflatable-cookie/longhorn`, `-poodle-svelte`, `-tauri` all at `0.2.0`,
  `latest`, MIT. Integrity equals the release run's packed artifact for all
  three.
- Rust crates are `publish = false`; the tag is their identity.
- Full record: `docs/logs/2026-09/22-longhorn-0-2-0-publication.md`.

**Runway.** `docs/roadmaps/g02/README.md`. No ready lanes. All four `0.2.0` lanes
(`g02.041`–`g02.044`) are complete and shipped. Parked and non-gating: `g02.036`
(production contextual agent tools — L1 merged, D1 blocked, production path
superseded by `g02.043`), `g02.008` (deferred), `g02.019` and `g02.017`
(Poodle/downstream, field evidence).

**Triage, both open and durable.** `docs/triage/20260829-154310-refresh-observations.md`
(handoff retention — keep, do not delete) and
`docs/triage/20260906-170000-bovine-production-mcp-request.md` (operator-authorised
consumer request; production settlement open, now largely answered by `g02.043`).

**New this thread.** `docs/triage/20260922-233429-agent-file-selection-gap.md`
carries the file-selection finding, its evidence, and the four open decisions.

**Papercuts.** Fifteen open in `PAPERCUTS.md`, two added this thread — the
release gate selector and the hardcoded release version. Both are worth reading
before the next release.

**Downstream.** acowtancy `g05.179` (Desktop updater host and Settings
experience) is in flight on the released artifact; `g05.180` (packaged private
macOS upgrade proof) is queued behind it. Poodle sweeps and the other consumer
adoption lanes are dispatched; none currently blocks Longhorn.

## Boundaries

- **The operator owns every release decision.** No tag, no npm publish, no
  Oracle notification without their word. An upstream request is never release
  approval.
- **Do not start the file-selection implementation.** The boundary question
  below is unresolved, and it changes which package owns the seam. Bring the
  answer back before opening a card.
- **Cross-repo writes need authorization.** The working rule: if the target
  project has a Paseo Chatterbox, send a brief; otherwise write a roadmap file
  and dispatch a Queue task.
- **Do not edit `.github/workflows/` without explicit human approval.**
- **Longhorn owns generic mechanisms; consumers own product policy.** Consumer
  apps own schemas, catalogues, commands, resources, workflows, and
  authorization. Poodle owns visual primitives — integrate through public APIs,
  never fork.
- **Pre-1.0, so no compatibility aliases, shims, or silent fallbacks.** The one
  exception in the tree is the temporary `dev` feature forward (below).
- Treat `docs/` as the authority. This handoff is transport, not the spine.

## Important Context

**The file-selection gap — the live question.** The operator wants an agent to be
able to drive a file selection over MCP without a native picker opening and
waiting for a human. The full finding is in
`docs/triage/20260922-233429-agent-file-selection-gap.md`; read it before saying
anything about this lane. The short version:

- Contract 022 explicitly makes native dialogs out of scope and assigns the mock
  dialog responder seam to the app; memo 024 deferred that seam's shape "until a
  consumer needs one". This is that need, so the deferral is due.
- The contract's offered route — `command` — does not work in practice. The
  picker is a UI-local call inside a component handler, not a registered command,
  and figmatic has no command registry at all. Figmatic recorded this dead end on
  2026-09-15 in `figmatic/PAPERCUTS.md`.
- Reconnaissance found the dominant mechanism is not a native dialog and not a
  web file input: it is `@tauri-apps/plugin-dialog`'s `open`/`save` called from
  JS, across acowtancy (~31 sites), nucleus (~9), figmatic (5), finch (3).
  Because `open()` returns **paths**, answering it needs no byte bridge and the
  consumer's downstream code is untouched. That is what makes a first slice
  cheap. Only `<input type="file">` (soundcheck, one site) needs real `File`
  objects materialised.
- The recommendation is a **Longhorn-owned selection-request seam**: a pending
  request published on the existing push surface beside
  `console`/`page-error`/`navigation`, answered by new `answer_selection` /
  `reject_selection` tools, with a drop-in `open`/`save` replacement in
  `@inflatable-cookie/longhorn` that leaves the human path bit-identical. The
  honesty line for the amendment: Longhorn still never drives the OS panel; the
  app's *selection* becomes answerable.
- **Four decisions are the operator's**: (1) does Longhorn own the seam, or does
  it stay app-owned per consumer; (2) is `save` in the first slice, given a save
  target is a write reach; (3) packaged or dev-only; (4) is `<input type="file">`
  a separate later slice. A fifth, smaller one: which consumer is the acceptance
  target — acowtancy has the most call sites, figmatic has the recorded blocker.
- Non-negotiable if this proceeds: no default path, no silent fallback, and an
  unanswered request expires *typed*. A picker already on screen cannot be
  retroactively answered.

**The partial-publish incident — read this before the next release.** The first
`0.2.0` publish run failed at the second of three packages. npm answered a
trusted-publisher mismatch with **404, not 403**, and the publish loop's `set -e`
had already let the first package through, so `@inflatable-cookie/longhorn@0.2.0`
landed while the two adapters did not. Propagation then made the state read as
ambiguous for several minutes — the run log said published while the packument
still showed only `0.1.0`. Recovery: the operator fixed the two adapters'
trusted publishers, the publish step gained an idempotency guard (skip a package
whose exact version is already on the registry), and the re-dispatch completed
it. No consumer was affected, because the `0.1.0` adapters peer on
`@inflatable-cookie/longhorn@0.1.0`, which stays on the registry. The lesson:
**a partial publish is a real failure mode, and the fix belongs in the workflow,
not in a manual retry.**

**Two release-tooling gaps are open as papercuts.** `effigy release:gates` runs
only three of the seven declared gates — it does not run the `[release.gates]`
table — which is exactly why local "all gates green" runs missed the version
drift that the CI runner caught. Also, the release version is hardcoded across
the artifact-proof scripts, so a version bump stales them by hand. Both are in
`PAPERCUTS.md` with the detail.

**One deliberate debt.** `crates/longhorn-tauri-agent-control/Cargo.toml` carries
`dev = ["agent-control-evaluate"]` as a migration forward for the 2026-09-22
rename, so consumers pinned to `0.1` can link against the `0.2.0` candidate
before they rename their own forwarding. It is temporary by construction; remove
it at the next breaking release.

**Where to start reading.** `docs/README.md`,
`docs/roadmaps/generation-index.md`, `docs/roadmaps/g02/README.md`,
`docs/contracts/contract-index.md`, `docs/architecture/system-architecture.md`,
and the release log above. `AGENTS.md` is the working contract and it is not
optional.

## Suggested Next Move

Start by asking the operator the four decisions in the file-selection triage note
— they are the whole blocker, and they are small, concrete questions rather than
open-ended design. Read
`docs/triage/20260922-233429-agent-file-selection-gap.md` first so you can argue
the recommendation rather than relay it.

Two things worth raising while you have their attention, because they are cheap
now and expensive later:

- The release-tooling papercuts. `effigy release:gates` claiming to be the gate
  set while running three of seven is the kind of gap that only shows up in CI,
  which is where it showed up this time.
- Whether `g02.036`'s D1 remainder should be formally closed. It is described as
  blocked and superseded by `g02.043`; a parked lane that is really finished
  should stop looking like work.

Once the four decisions are answered, the natural shape is a contract 022
amendment plus one `g02` card for the first slice, with the consumer acceptance
target named up front. The reconnaissance is done, so that card can start at
implementation.

If the operator would rather not decide yet, the honest alternative is to leave
the gap open and say so — but do not start building against an unresolved
boundary.

## Completion Protocol

**Stay read-only until you receive `Ownership transfer complete`.** The source
Chatterbox is still the authority for this lane until the transfer is confirmed;
do not open cards, amend contracts, dispatch tasks, or touch any repository
surface before then.

When you receive it, re-enter Chatterbox mode per the northstar skill and reload
current `main` — this handoff is context, not durable authority. Verify the
state above against the repository before trusting it: the release log, the
runway, the triage notes, and `git log` at the HEAD named in Current State.

Then continue the lane from `## Suggested Next Move`. Planning state lives on
`docs/roadmaps/g02/README.md` and `docs/roadmaps/generation-index.md`; new work
becomes a `g02` card plus the governing contract change. Unresolved risks are
named in Current State — chiefly the four file-selection decisions, the two
release-tooling papercuts, and the temporary `dev` forward that must be removed
at the next breaking release.

Queue preflight found no unfinished tasks attached to the source Chatterbox, so
there is no attention to inherit and no task to continue by default. If the
operator authorizes the file-selection lane, dispatch goes through the
northstar-queue plugin as usual — planning approval and execution authorization
are separate, and a promoted contract or card is not execution approval.
