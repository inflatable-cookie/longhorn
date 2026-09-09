# Generation Index

## Mode

Dependency-led. New generations start only after a shared Longhorn gap is
characterized and promoted through the docs spine.

Northstar now has one execution level. The generation README owns the roadmap
and approved frontier; `docs/roadmaps/gNN/NNN-<slug>.md` is the sole executable
planning unit, referenced as `gNN.NNN`. There are no milestone wrappers and no
nested `batch-cards/` hierarchy: the flattened-task migration absorbed every
card record into its owning task (or, for Card 177, into new task g02.037) and
compacted g01 into `archive/g01.md`.

## Active Generation

[g02](g02/README.md) — workspace integrity remediation, now open past it.
Research memo 018 characterizes the post-g01 audit gap; Cards 138-147
delivered all six remediation tasks. Research memo 019 characterizes
the in-app update gap and compiles contract 018; g02.009 (Cards 150-154) is
the first non-remediation task. Research memo 020 compiles contract 019
over licensing as g02.010 (Cards 155-158).

## Rollover History

- [g01](archive/g01.md) — complete, 20 tasks, Cards 001-137. Roll-up replaces
  the removed expanded tree; git history is the full-fidelity archive.

## Next Task

**Current operator frontier:** [g02.036 L1](g02/036-production-contextual-agent-tools.md)
provider-free typed dispatcher fixture is ready. Coordinator dispatches L1
first; D1 waits for independent acceptance and merge. Prior adoption narrative
below remains historical context, not a competing automatic dispatch.

One live pointer lives here, updated in the same commit that closes a task —
the running narrative that used to fill this section drifted into three
competing pointers, which is how Card 216 came to exist. Task closeout detail
lives on the tasks themselves.

**Live:** the agent-control arc is complete in Longhorn (g02.029-033,
Cards 227-237, PRs 2-6, all 2026-08-19). Next: per-app adoption — each
consumer repo compiles one task: compose per
`docs/guides/agent-control-composition.md`, then
`effigy agent-control:install-skill -- <repo>` (from the Longhorn
checkout). Soundcheck adopted first (2026-08-20, its
`docs/logs/2026-08/20-agent-control-adoption.md`) and confirmed the
`claude mcp add` paste-line end to end — the adoption kit's last open
review item. Figmatic is in flight on the `NoCommandBridge` composition.
Figmatic's screenshot blocker cleared with g02.034 (PR 7,
`e135d725`); its PR 14 reruns on that revision. Child-webview semantic
targeting is merged:
[g02.035](g02/035-agent-control-child-webview-targeting.md) (Cards
239-240, PR 8, `592dbdb8`) — an app opts in child labels and agents
drive them directly. Figmatic's full path is now open: rerun PR 14 on
this revision, `.with_semantic_child("figmatic-preview")` at mount,
re-run the skill installer, automate preview acceptance (never opt in
`longhorn-browser` views). Three apps remain after Figmatic; operator
picks order.
Still open on the operator side: return the promoted Longhorn commit to
Bovine Card 128 adoption (from g02.028).

Production contextual app-tool planning is separately captured as g02.036
scope D1. It must not start from the dev-only contract 022 or duplicate
Swallowtail's shared transport lane. Longhorn settles its typed dispatch and
validation surface; Swallowtail settles the bridge/profile and route matrix;
Desktop spec010 at `30a338f2` is the canonical context-disclosure and admission
record. Planning ownership is aligned; promotion remains held for Contract060
delivery evidence, exact-head reviews, and the final bilateral cross-check.

The memo-023 release-audit suite — tasks g02.021-027, cards 200-224 — is
complete except Card 218's remaining release machinery, noted below.

- Complete: g02.021 (200-203), g02.022 (204-206), g02.023 (207-210, 224),
  g02.024 (211-214), g02.025 (215-217), g02.027 (221-223).
- Card 224 was opened by review of the suite itself: Card 210's identity
  generation had a first-run race, and the compare-and-swap that would close
  it does not exist on any platform backend. The refusal and the narrowing
  that replaced it are recorded on the trait and in contract 004.
- Operator decisions of 2026-08-15 landed: 210 (store vocabulary moved to
  `longhorn-core`) and the Card 214 sidebar-label divergence (Svelte side
  correct; Rust aligned). Only 218 remains open, and it is no longer held.
- The `.github/workflows/` items in 219/220 landed 2026-08-15 with approval.
- g02.026 is in progress on Card 218 only, which is part-landed. Its Poodle
  precondition is met at public 0.2.2, and g16.008 removed the linked-Poodle
  exemption (step 5), which unblocks g02.014. Step 1 — a release gate on the
  `linkedPoodleAccepted` flag — is obsolete with the flag. Steps 2-4 are
  release hardening and need recompiling before they are ready.
- No Longhorn task auto-starts from Card 225.
- Card 226 (2026-08-17) closed a mixed-scale coordinate gap raised by Figmatic
  consumer evidence: `LogicalLayoutMapper` converts each object through its own
  scale, valid on macOS and Linux and excluded on Windows. Contract 009's
  blanket ban on per-monitor division now states the per-platform truth.
  Figmatic adoption is the next task and belongs to the Figmatic thread.

Pre-suite lanes are tracked on the [g02 runway](g02/README.md). g02.015 closed
2026-08-17 and g02.016 closed 2026-08-12; g02.017 and g02.019 are the
in-progress lanes, g02.020 is ready, and g02.037 (focused panel surfaces,
promoted from independent Card 177) is in progress. Absorbed records: Card
166 (ready) inside g02.014, Card 160 (complete) inside g02.011, Card 149
(blocked) inside g02.008, Card 218 (needs recompile) inside g02.026.
Collapsed with no new task: Cards 178/179/180 and 226 (see the g02 runway).
Open pre-suite work is 166 (ready, in g02.014), 177 (in progress, now
g02.037), and 218 (needs recompile, in g02.026). Card 149 is blocked in
g02.008.

## Memo 023 Release-audit Suite

Research memo 023 (full-repo release audit, 2026-08-14) compiles into
tasks [g02.021-027](g02/README.md) — cards 200-223, the suite that is
the live pointer above. Its state lives there; this section stays as the
memo-to-roadmap provenance.
